use std::time::Duration;

use akl_api::{AppState, app, config::Config};
use akl_persistence::Database;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        // Für Container-Healthchecks im Distroless-Image (kein curl/Shell).
        std::process::exit(match healthcheck(config.bind.port()).await {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("healthcheck fehlgeschlagen: {err}");
                1
            }
        });
    }
    init_tracing(config.log_json);
    tracing::info!(
        ?config,
        "AI-Keyless API startet (inoffizielles, unabhängiges Projekt)"
    );

    let db = match &config.database_url {
        Some(url) => {
            let db = Database::connect_lazy(url)?;
            tokio::spawn(migrate_with_retry(db.clone()));
            Some(db)
        }
        None => {
            tracing::warn!("AKL_DATABASE_URL nicht gesetzt – Datenbank: not_configured");
            None
        }
    };

    let router = app(AppState { db }, config.web_dir.as_deref());
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %config.bind, "lauscht");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn healthcheck(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fut = async {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port)).await?;
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await?;
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await?;
        Ok::<_, std::io::Error>(buf)
    };
    let buf = tokio::time::timeout(Duration::from_secs(3), fut).await??;
    if buf.starts_with(b"HTTP/1.1 200") {
        Ok(())
    } else {
        Err("unerwartete Antwort".into())
    }
}

fn init_tracing(json: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,sqlx=warn"));
    let builder = fmt().with_env_filter(filter);
    if json {
        builder.json().init();
    } else {
        builder.init();
    }
}

/// Migriert im Hintergrund, damit die API auch bei verzögert startender
/// Datenbank erreichbar bleibt und den Zustand als `degraded` meldet.
async fn migrate_with_retry(db: Database) {
    let mut delay = Duration::from_secs(1);
    loop {
        match db.migrate().await {
            Ok(()) => {
                tracing::info!("Datenbankmigrationen angewendet");
                return;
            }
            Err(err) => {
                tracing::warn!(error = %err, retry_in = ?delay, "Migration fehlgeschlagen");
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(60));
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Shutdown-Signal empfangen");
}
