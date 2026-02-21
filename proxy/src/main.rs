mod control;
mod models;
mod proxy;
mod rewrite;
mod rules;
mod state;

use crate::state::{AppState, ProxyConfig};
use anyhow::Context;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,hyper=warn".into()),
        )
        .init();

    let proxy_port = read_port("INTERCEPTKIT_PROXY_PORT", 8081);
    let control_port = read_port("INTERCEPTKIT_CONTROL_PORT", 4592);

    let state = Arc::new(AppState::new(ProxyConfig {
        proxy_port,
        control_port,
    }));

    let proxy_addr = SocketAddr::from(([127, 0, 0, 1], proxy_port));
    let control_addr = SocketAddr::from(([127, 0, 0, 1], control_port));

    info!("starting interceptkit-proxy");
    info!("control api: http://{}", control_addr);
    info!("proxy listener: {}", proxy_addr);

    let control_router = control::router(state.clone());
    let control_listener = TcpListener::bind(control_addr)
        .await
        .with_context(|| format!("failed to bind control API on {}", control_addr))?;

    let control_server = async move {
        axum::serve(control_listener, control_router)
            .await
            .context("control API stopped unexpectedly")
    };

    let proxy_server = proxy::run_proxy_listener(proxy_addr, state.clone());

    tokio::select! {
        result = control_server => result,
        result = proxy_server => result,
    }
}

fn read_port(env_key: &str, default_port: u16) -> u16 {
    std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(default_port)
}
