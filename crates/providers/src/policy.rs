//! Netzwerkrichtlinie gegen SSRF für frei konfigurierbare Provider-URLs.
//!
//! Lokale Modellserver im LAN sind der Hauptanwendungsfall, daher sind private
//! Adressen standardmäßig erlaubt. Immer gesperrt sind Ziele, die nie ein
//! Modellserver sein sollten: Link-Local (u. a. Cloud-Metadaten 169.254.169.254),
//! unspezifizierte, Multicast- und Broadcast-Adressen. Loopback nur per Opt-in.
//! Der Hostname wird genau einmal aufgelöst; verbunden wird nur mit den geprüften
//! Adressen (kein DNS-Rebinding zwischen Prüfung und Verbindung).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use url::Url;

use crate::ProviderError;

#[derive(Debug, Clone)]
pub struct NetworkPolicy {
    pub allow_loopback: bool,
    pub allow_private: bool,
    /// Falls gesetzt, sind nur diese Hostnamen (exakt, klein geschrieben) erlaubt.
    pub allowed_hosts: Option<Vec<String>>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allow_loopback: false,
            allow_private: true,
            allowed_hosts: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    pub url: Url,
    pub host: String,
    pub addrs: Vec<SocketAddr>,
}

/// Syntaktische Prüfung ohne Netzwerkzugriff (z. B. beim Speichern einer Konfiguration).
pub fn parse_base_url(raw: &str, policy: &NetworkPolicy) -> Result<(Url, String), ProviderError> {
    let url = Url::parse(raw.trim()).map_err(|_| ProviderError::InvalidUrl)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ProviderError::InvalidUrl);
    }
    if !url.username().is_empty() || url.password().is_some() {
        // Zugangsdaten gehören in Secret-Dateien, nicht in URLs.
        return Err(ProviderError::InvalidUrl);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(ProviderError::InvalidUrl);
    }
    let host = url
        .host_str()
        .ok_or(ProviderError::InvalidUrl)?
        .to_ascii_lowercase();
    if let Some(allowed) = &policy.allowed_hosts
        && !allowed.iter().any(|h| h.eq_ignore_ascii_case(&host))
    {
        return Err(ProviderError::BlockedAddress);
    }
    // Literale IP-Adressen sofort prüfen.
    if let Some(ip) = literal_ip(&url) {
        check_ip(ip, policy)?;
    }
    Ok((url, host))
}

fn literal_ip(url: &Url) -> Option<IpAddr> {
    match url.host()? {
        url::Host::Ipv4(v4) => Some(IpAddr::V4(v4)),
        url::Host::Ipv6(v6) => Some(IpAddr::V6(v6)),
        url::Host::Domain(_) => None,
    }
}

/// Löst den Host auf und prüft jede Adresse. Schlägt fehl, wenn auch nur eine
/// Adresse gesperrt ist (ein Angreifer könnte sonst gemischte Antworten nutzen).
pub async fn resolve(raw: &str, policy: &NetworkPolicy) -> Result<Target, ProviderError> {
    let (url, host) = parse_base_url(raw, policy)?;
    let port = url
        .port_or_known_default()
        .ok_or(ProviderError::InvalidUrl)?;
    let addrs: Vec<SocketAddr> = match literal_ip(&url) {
        Some(ip) => vec![SocketAddr::new(ip, port)],
        None => tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| ProviderError::Dns)?
            .collect(),
    };
    if addrs.is_empty() {
        return Err(ProviderError::Dns);
    }
    for addr in &addrs {
        check_ip(addr.ip(), policy)?;
    }
    Ok(Target { url, host, addrs })
}

pub fn check_ip(ip: IpAddr, policy: &NetworkPolicy) -> Result<(), ProviderError> {
    let ip = canonical(ip);
    let blocked = match ip {
        IpAddr::V4(v4) => {
            v4.is_unspecified()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || (v4.is_loopback() && !policy.allow_loopback)
                || (is_private_v4(v4) && !policy.allow_private)
        }
        IpAddr::V6(v6) => {
            v6.is_unspecified()
                || v6.is_multicast()
                || is_link_local_v6(v6)
                || (v6.is_loopback() && !policy.allow_loopback)
                || (is_unique_local_v6(v6) && !policy.allow_private)
        }
    };
    if blocked {
        Err(ProviderError::BlockedAddress)
    } else {
        Ok(())
    }
}

/// IPv4-gemappte IPv6-Adressen (::ffff:a.b.c.d) wie IPv4 behandeln.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        v4 => v4,
    }
}

fn is_private_v4(ip: Ipv4Addr) -> bool {
    // RFC 1918 + Carrier-Grade-NAT (RFC 6598)
    ip.is_private() || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xc0) == 64)
}

fn is_link_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn is_unique_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> NetworkPolicy {
        NetworkPolicy::default()
    }

    #[test]
    fn always_blocked_targets() {
        for ip in [
            "169.254.169.254",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "fe80::1",
            "::",
            "ff02::1",
            "::ffff:169.254.169.254",
        ] {
            let ip: IpAddr = ip.parse().unwrap();
            let open = NetworkPolicy {
                allow_loopback: true,
                allow_private: true,
                allowed_hosts: None,
            };
            assert!(check_ip(ip, &open).is_err(), "{ip}");
        }
    }

    #[test]
    fn loopback_and_private_follow_policy() {
        let lo: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(check_ip(lo, &p()).is_err());
        assert!(
            check_ip(
                lo,
                &NetworkPolicy {
                    allow_loopback: true,
                    ..p()
                }
            )
            .is_ok()
        );
        assert!(check_ip("::ffff:127.0.0.1".parse().unwrap(), &p()).is_err());

        for private in [
            "192.168.1.20",
            "10.0.0.5",
            "172.16.3.4",
            "100.64.0.1",
            "fd00::5",
        ] {
            let ip: IpAddr = private.parse().unwrap();
            assert!(check_ip(ip, &p()).is_ok(), "{private}");
            assert!(
                check_ip(
                    ip,
                    &NetworkPolicy {
                        allow_private: false,
                        ..p()
                    }
                )
                .is_err(),
                "{private}"
            );
        }
        assert!(check_ip("8.8.8.8".parse().unwrap(), &p()).is_ok());
    }

    #[test]
    fn url_syntax() {
        assert!(parse_base_url("http://ollama.lan:11434", &p()).is_ok());
        assert!(parse_base_url("https://lm.lan/v1", &p()).is_ok());
        for bad in [
            "ftp://x",
            "file:///etc/passwd",
            "http://user:pw@host",
            "http://host/?a=1",
            "http://host/#x",
            "kein url",
            "http://169.254.169.254/latest/meta-data",
            "http://127.0.0.1:11434",
        ] {
            assert!(parse_base_url(bad, &p()).is_err(), "{bad}");
        }
    }

    #[test]
    fn host_allowlist() {
        let policy = NetworkPolicy {
            allowed_hosts: Some(vec!["ollama.lan".into()]),
            ..p()
        };
        assert!(parse_base_url("http://OLLAMA.lan:11434", &policy).is_ok());
        assert!(parse_base_url("http://other.lan", &policy).is_err());
    }

    #[tokio::test]
    async fn resolution_checks_every_address() {
        // localhost löst auf Loopback auf → ohne Opt-in gesperrt.
        let err = resolve("http://localhost:11434", &p()).await.unwrap_err();
        assert_eq!(err.code(), "blocked_address");
        let open = NetworkPolicy {
            allow_loopback: true,
            ..p()
        };
        let target = resolve("http://localhost:11434", &open).await.unwrap();
        assert!(target.addrs.iter().all(|a| a.ip().is_loopback()));
    }
}
