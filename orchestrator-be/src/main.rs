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
        &config.bitcoin_rpc_user,
        &config.bitcoin_rpc_pass,
    ));
    warn_without_txindex(btc_client.clone());

    let state = if let Some(database_url) = &config.database_url {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .idle_timeout(std::time::Duration::from_secs(300))
            .max_lifetime(std::time::Duration::from_secs(1800))
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
            config.proposal_expiry_days,
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
            config.proposal_expiry_days,
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

/// A node without `-txindex` cannot find a mined reveal, so the orchestrator cannot tell which of a
/// proposal's commit + reveal pairs was mined first and takes the desktop's report unchecked (#605). Checked once in the background so a slow node does not hold
/// back startup; the operator sees it in the log.
fn warn_without_txindex(btc_client: Arc<infrastructure::bitcoin_rpc::HttpBitcoinRpcClient>) {
    tokio::spawn(async move {
        use infrastructure::bitcoin_rpc::TxIndex;
        match btc_client.txindex().await {
            Ok(TxIndex::Synced) => {
                tracing::info!("bitcoin node keeps a transaction index (-txindex)")
            }
            Ok(TxIndex::Syncing) => tracing::warn!(
                "bitcoin node is still building its transaction index: until it is synced, the \
                 reconcile cannot confirm a proposal with more than one commit + reveal pair"
            ),
            Ok(TxIndex::Missing) => tracing::error!(
                "bitcoin node runs without -txindex: for a proposal with more than one commit + \
                 reveal pair the orchestrator cannot check which was mined first and takes the \
                 desktop's report; restart bitcoind with -txindex=1"
            ),
            Err(e) => tracing::warn!(error = %e, "could not check the bitcoin node for -txindex"),
        }
    });
}
