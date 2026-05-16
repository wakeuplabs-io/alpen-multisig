use anyhow::Context;
use axum::{http::Method, Router};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod application;
mod config;
mod domain;
mod error;
mod handlers;
mod infrastructure;
mod state;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env().context("failed to load config")?;

    let btc_client = Arc::new(infrastructure::bitcoin_rpc::HttpBitcoinRpcClient::new(
        &config.bitcoin_rpc_url,
        config.bitcoin_wallet_name.as_deref(),
        &config.bitcoin_rpc_user,
        &config.bitcoin_rpc_pass,
    ));

    let state = if let Some(database_url) = &config.database_url {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .context("failed to connect to postgres")?;
        sqlx::migrate!()
            .run(&pool)
            .await
            .context("failed to run database migrations")?;
        let repo = Arc::new(infrastructure::postgres_repo::PostgresProposalRepository::new(pool));
        state::AppState::new(
            repo,
            config.strata_admin_state_rpc_url.clone(),
            config.auth_challenge_ttl_ms,
            config.auth_session_ttl_ms,
            btc_client,
        )
    } else {
        tracing::warn!("DATABASE_URL not set — using in-memory storage (data will not persist)");
        let repo = Arc::new(infrastructure::memory_repo::InMemoryProposalRepository::new());
        state::AppState::new(
            repo,
            config.strata_admin_state_rpc_url.clone(),
            config.auth_challenge_ttl_ms,
            config.auth_session_ttl_ms,
            btc_client,
        )
    };

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers(Any)
        .allow_origin(Any);

    let app = Router::new()
        .nest("/api/v1", handlers::router(state))
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("failed to bind")?;

    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
