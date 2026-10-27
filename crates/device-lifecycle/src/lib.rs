//! Geräte-Lifecycle (AKL-M040): persistente Identität je Profil,
//! Zustandsdienst mit Kill-Switch, Discovery-Antwort, Adoption und
//! Kontrollkanal (Keepalive/Reconnect).

pub mod adoption;
pub mod control;
pub mod discovery;
pub mod identity;
pub mod service;
