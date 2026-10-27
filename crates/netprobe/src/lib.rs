//! Netzwerkprobe für AKL-M080-01 (nur Standardbibliothek).
//!
//! Misst, was ein Docker-Netzmodus (bridge/host/macvlan) für ein LAN-Gerät
//! bedeutet: Kommt ein UDP-Broadcast aus dem LAN im Container an? Ist ein
//! TCP-Port des Containers aus dem LAN erreichbar, und mit welcher
//! Absenderadresse sieht der Container die Gegenstelle?
//!
//! Protokollneutral: kein Protect-/Ubiquiti-Protokoll, nur eine eigene
//! Kennung (`AKL-NETPROBE`).

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub const MAGIC: &str = "AKL-NETPROBE";

/// Ein beobachtetes Ereignis des Empfängers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Udp { from: SocketAddr, payload: String },
    Tcp { from: SocketAddr },
}

impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Udp { from, payload } => write!(f, "udp from={from} payload={payload}"),
            Event::Tcp { from } => write!(f, "tcp from={from}"),
        }
    }
}

/// Startet UDP- und/oder TCP-Empfänger auf `0.0.0.0` und liefert Ereignisse
/// über den Kanal. Fremde UDP-Pakete (ohne Kennung) werden ignoriert.
pub fn listen(
    udp_port: Option<u16>,
    tcp_port: Option<u16>,
) -> std::io::Result<mpsc::Receiver<Event>> {
    let (tx, rx) = mpsc::channel();
    if let Some(port) = udp_port {
        let sock = UdpSocket::bind(("0.0.0.0", port))?;
        let tx = tx.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 512];
            while let Ok((n, from)) = sock.recv_from(&mut buf) {
                let payload = String::from_utf8_lossy(&buf[..n]).trim().to_owned();
                if payload.starts_with(MAGIC) && tx.send(Event::Udp { from, payload }).is_err() {
                    break;
                }
            }
        });
    }
    if let Some(port) = tcp_port {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Ok(from) = stream.peer_addr() else {
                    continue;
                };
                // Die Gegenstelle erfährt, welche Adresse der Empfänger sieht.
                let _ = writeln!(stream, "{MAGIC} seen={from}");
                if tx.send(Event::Tcp { from }).is_err() {
                    break;
                }
            }
        });
    }
    Ok(rx)
}

/// Sendet `count` Broadcast-/Unicast-Datagramme mit Kennung an `to`.
pub fn send_udp(to: SocketAddr, count: u32, tag: &str) -> std::io::Result<()> {
    let sock = UdpSocket::bind(("0.0.0.0", 0))?;
    sock.set_broadcast(true)?;
    for i in 0..count {
        sock.send_to(format!("{MAGIC} {tag} #{i}").as_bytes(), to)?;
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// Baut eine TCP-Verbindung auf und liefert die vom Empfänger gesehene
/// Absenderadresse (Banner `AKL-NETPROBE seen=…`).
pub fn connect(target: &str, timeout: Duration) -> std::io::Result<String> {
    let addr = target
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::other("Adresse nicht auflösbar"))?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    let mut buf = String::new();
    let deadline = Instant::now() + timeout;
    let mut chunk = [0u8; 128];
    while !buf.contains('\n') && Instant::now() < deadline {
        match stream.read(&mut chunk)? {
            0 => break,
            n => buf.push_str(&String::from_utf8_lossy(&chunk[..n])),
        }
    }
    let line = buf.lines().next().unwrap_or_default();
    line.strip_prefix(&format!("{MAGIC} seen="))
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other(format!("unerwartete Antwort: {line:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn tcp_roundtrip_reports_peer_address() {
        let port = free_port();
        let rx = listen(None, Some(port)).unwrap();
        let seen = connect(&format!("127.0.0.1:{port}"), Duration::from_secs(2)).unwrap();
        assert!(seen.starts_with("127.0.0.1:"), "{seen}");
        let event = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(event, Event::Tcp { from } if from.ip().is_loopback()));
    }

    #[test]
    fn udp_unicast_with_magic_is_received() {
        let port = UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let rx = listen(Some(port), None).unwrap();
        send_udp(([127, 0, 0, 1], port).into(), 1, "test").unwrap();
        let event = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(
            matches!(event, Event::Udp { ref payload, .. } if payload == "AKL-NETPROBE test #0")
        );
    }

    #[test]
    fn foreign_udp_is_ignored() {
        let port = UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let rx = listen(Some(port), None).unwrap();
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        s.send_to(b"hello", ("127.0.0.1", port)).unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());
    }

    #[test]
    fn connect_to_closed_port_fails() {
        let port = free_port();
        assert!(connect(&format!("127.0.0.1:{port}"), Duration::from_secs(1)).is_err());
    }
}
