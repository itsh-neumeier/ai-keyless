//! Simulierter Controller als eigenständiger Prozess (z. B. im Docker-Testnetz).
//! Nutzung: akl-protect-sim [BIND] [normal|silent|reset]

use akl_protect_sim::{Behavior, SimController};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let bind = args.next().unwrap_or_else(|| "0.0.0.0:7443".to_owned());
    let behavior = match args.next().as_deref() {
        None | Some("normal") => Behavior::Normal,
        Some("silent") => Behavior::Silent,
        Some("reset") => Behavior::Reset,
        Some(other) => {
            eprintln!("unbekanntes Verhalten: {other}");
            std::process::exit(2);
        }
    };
    let sim = SimController::start_on(&bind, behavior).await?;
    println!(
        "akl-protect-sim (synthetisch) lauscht auf {} – {behavior:?}",
        sim.addr
    );
    println!("SHA-256-Fingerprint: {}", sim.fingerprint);
    tokio::signal::ctrl_c().await?;
    Ok(())
}
