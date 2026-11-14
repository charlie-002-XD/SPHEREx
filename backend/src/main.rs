//! Binary entry point (task b5): config -> IRSA client -> optional
//! cache warm-up -> HTTP server with graceful shutdown.
//!
//! `.expect()` is used only here, at bootstrap: a failure before the
//! listener binds is fatal anyway and the message goes to stderr.

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

use spherex_backend::api::{build_router, targets, AppState};
use spherex_backend::config::Config;
use spherex_backend::irsa::IrsaClient;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env().expect("cannot load configuration");
    let client = IrsaClient::new(cfg.irsa.clone()).expect("cannot build IRSA client");

    if cfg.warm_targets {
        let warm_client = client.clone();
        let targets = targets::warm_targets().expect("invalid preset target table");
        tokio::spawn(async move {
            let report = warm_client.warm(&targets).await;
            info!(?report, "cache warm-up finished");
        });
    }

    let state = AppState::from_config(client, &cfg);
    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    let listener = TcpListener::bind(addr)
        .await
        .expect("cannot bind listen socket");
    info!(%addr, static_dir = %cfg.static_dir.display(), "listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server terminated with an error");
}

/// Graceful shutdown on SIGINT or SIGTERM (Unix); SIGINT only elsewhere.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("cannot listen for SIGINT: {e}");
        }
    };
    #[cfg(unix)]
    {
        let term = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut s) => {
                    s.recv().await;
                }
                Err(e) => tracing::error!("cannot listen for SIGTERM: {e}"),
            }
        };
        tokio::select! {
            () = ctrl_c => {},
            () = term => {},
        }
    }
    #[cfg(not(unix))]
    ctrl_c.await;
    info!("shutdown requested");
}
