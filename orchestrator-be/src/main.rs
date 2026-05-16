use anyhow::Context;
use axum::{http::Method, Router};
use bitcoin::key::UntweakedKeypair;
use bitcoin::secp256k1::{SecretKey, SECP256K1};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use strata_l1_txfmt::MagicBytes;
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

    // Build operator keypair from config secret key.
    let sk_bytes = hex::decode(&config.operator_secret_key_hex)
        .context("OPERATOR_SECRET_KEY_HEX must be valid hex")?;
    let sk = SecretKey::from_slice(&sk_bytes)
        .context("OPERATOR_SECRET_KEY_HEX must be a valid 32-byte secp256k1 secret key")?;
    let operator_keypair = UntweakedKeypair::from_secret_key(SECP256K1, &sk);

    // Parse magic bytes from config.
    let magic_raw = hex::decode(&config.bitcoin_magic_bytes_hex)
        .context("BITCOIN_MAGIC_BYTES_HEX must be valid hex")?;
    let magic_arr: [u8; 4] = magic_raw.try_into().map_err(|_| {
        anyhow::anyhow!("BITCOIN_MAGIC_BYTES_HEX must be exactly 4 bytes (8 hex chars)")
    })?;
    let bitcoin_magic_bytes = MagicBytes::new(magic_arr);

    let bitcoin_network = config.bitcoin_network;

    // Build Bitcoin RPC client.
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
            operator_keypair,
            config.confirm_poll_interval_ms,
            config.confirm_timeout_ms,
            bitcoin_magic_bytes,
            bitcoin_network,
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
            operator_keypair,
            config.confirm_poll_interval_ms,
            config.confirm_timeout_ms,
            bitcoin_magic_bytes,
            bitcoin_network,
        )
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
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
