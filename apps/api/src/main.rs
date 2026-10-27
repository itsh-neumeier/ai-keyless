use std::time::Duration;

use akl_api::{AppState, app, auth, config::Config, crypto};
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
    if std::env::args().nth(1).as_deref() == Some("setup-token") {
        std::process::exit(match issue_setup_token(&config).await {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("Fehler: {err}");
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
            tokio::spawn(cleanup_sessions(db.clone()));
            Some(db)
        }
        None => {
            tracing::warn!("AKL_DATABASE_URL nicht gesetzt – Datenbank: not_configured");
            None
        }
    };

    let mut state = AppState::new(db, config.cookie_secure);
    state.provider_policy = akl_providers::NetworkPolicy {
        allow_loopback: config.provider_allow_loopback,
        allow_private: config.provider_allow_private,
        allowed_hosts: config.provider_allowed_hosts.clone(),
    };
    state.provider_timeout = Duration::from_secs(config.provider_timeout_secs);
    state.secrets_dir = config.secrets_dir.clone();
    state.jobs_per_hour = config.jobs_per_hour;
    if let Some(db) = state.db.clone() {
        tracing::info!(
            jobs_per_hour = state.jobs_per_hour,
            "Verarbeitungs-Worker gestartet"
        );
        tokio::spawn(akl_api::worker::Worker::new(state.clone(), db).run());
    }
    let router = app(state, config.web_dir.as_deref());
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    tracing::info!(addr = %config.bind, "lauscht");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Gibt ein Einrichtungs-Token auf stdout aus (nicht ins Log). Nur möglich,
/// solange noch kein Konto existiert; ältere offene Tokens werden ungültig.
async fn issue_setup_token(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let url = config
        .database_url
        .as_deref()
        .ok_or("AKL_DATABASE_URL bzw. AKL_DATABASE_URL_FILE ist nicht gesetzt")?;
    let db = Database::connect_lazy(url)?;
    db.migrate().await?;
    if db.user_count().await? > 0 {
        return Err("Einrichtung bereits abgeschlossen – es existiert schon ein Konto".into());
    }
    let token = crypto::random_token();
    db.replace_setup_token(
        &crypto::token_hash(&token),
        chrono::Utc::now() + auth::SETUP_TOKEN_TTL,
    )
    .await?;
    println!(
        "Einrichtungs-Token (einmalig, {} Minuten gültig):",
        auth::SETUP_TOKEN_TTL.num_minutes()
    );
    println!("{token}");
    Ok(())
}

/// Entfernt abgelaufene Sitzungen stündlich.
async fn cleanup_sessions(db: Database) {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        match db.delete_expired_sessions().await {
            Ok(n) if n > 0 => tracing::info!(removed = n, "abgelaufene Sitzungen entfernt"),
            Ok(_) => {}
            Err(err) => tracing::debug!(error = %err, "Sitzungsbereinigung übersprungen"),
        }
    }
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
