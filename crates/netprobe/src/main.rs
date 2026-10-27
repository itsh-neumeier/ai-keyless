//! `akl-netprobe` – Werkzeug für das Netzwerk-Lab (AKL-M080-01).
//!
//! ```text
//! akl-netprobe listen [--udp PORT] [--tcp PORT] [--seconds N]
//! akl-netprobe send   ADDR:PORT [--count N] [--tag TEXT]
//! akl-netprobe connect HOST:PORT [--timeout SECS]
//! ```

use std::{process::ExitCode, time::Duration};

use akl_netprobe::{connect, listen, send_udp};

fn flag<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("listen") => {
            let udp = flag::<u16>(args, "--udp");
            let tcp = flag::<u16>(args, "--tcp");
            if udp.is_none() && tcp.is_none() {
                return Err("--udp und/oder --tcp angeben".into());
            }
            let rx = listen(udp, tcp).map_err(|e| format!("bind: {e}"))?;
            println!("listening udp={udp:?} tcp={tcp:?}");
            let deadline = flag::<u64>(args, "--seconds").map(Duration::from_secs);
            let start = std::time::Instant::now();
            loop {
                let wait = match deadline {
                    Some(d) => match d.checked_sub(start.elapsed()) {
                        Some(rest) => rest,
                        None => return Ok(()),
                    },
                    None => Duration::from_secs(3600),
                };
                if let Ok(event) = rx.recv_timeout(wait) {
                    println!("{event}");
                }
            }
        }
        Some("send") => {
            let to = args
                .get(1)
                .and_then(|a| a.parse().ok())
                .ok_or("Ziel ADDR:PORT fehlt")?;
            let count = flag(args, "--count").unwrap_or(3);
            let tag: String = flag(args, "--tag").unwrap_or_else(|| "probe".into());
            send_udp(to, count, &tag).map_err(|e| format!("send: {e}"))?;
            println!("sent {count} to {to}");
            Ok(())
        }
        Some("connect") => {
            let target = args.get(1).ok_or("Ziel HOST:PORT fehlt")?;
            let timeout = Duration::from_secs(flag(args, "--timeout").unwrap_or(3));
            let seen = connect(target, timeout).map_err(|e| format!("connect: {e}"))?;
            println!("connected {target} seen_as={seen}");
            Ok(())
        }
        _ => Err("Befehl: listen | send | connect".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}
