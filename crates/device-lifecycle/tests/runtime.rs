//! Laufzeit der Geräte-Emulation gegen `akl-protect-sim` und eine echte
//! PostgreSQL (AKL-M040-07, synthetisch): Discovery → Adoption → online.

use std::{net::SocketAddr, time::Duration};

use akl_core::{DeviceProfileKind::*, DeviceState, DeviceState::*};
use akl_device_lifecycle::{
    adoption::{ControllerTrust, wire},
    control::ControlConfig,
    identity::IdentityStore,
    runtime::{EmulationConfig, supervise},
    service::Lifecycle,
};
use akl_persistence::testing::{fresh_schema, test_database_url};
use akl_protect_sim::{Behavior, SimController, discovery_probe, send_line_pinned};
use uuid::Uuid;

const TIMEOUT: Duration = Duration::from_secs(2);

/// Freie Ports ermitteln (kurz binden, wieder freigeben).
fn free_udp() -> SocketAddr {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

fn free_tcp() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

async fn state(lc: &Lifecycle, id: Uuid) -> DeviceState {
    lc.db().get_device_profile(id).await.unwrap().unwrap().state
}

async fn wait_for(lc: &Lifecycle, id: Uuid, target: DeviceState) {
    for _ in 0..150 {
        if state(lc, id).await == target {
            return;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    panic!(
        "{target:?} nicht erreicht, aktuell {:?}",
        state(lc, id).await
    );
}

/// Wiederholt die Probe, bis der Supervisor das Profil übernommen hat.
async fn probe_until_answered(addr: SocketAddr) -> Vec<u8> {
    for _ in 0..50 {
        if let Some(reply) = discovery_probe(addr, &[1, 0, 0, 0], Duration::from_millis(200))
            .await
            .unwrap()
        {
            return reply;
        }
        tokio::time::sleep(Duration::from_millis(1100)).await;
    }
    panic!("keine Discovery-Antwort");
}

#[tokio::test]
async fn supervisor_runs_discovery_adoption_and_control_and_stops_on_kill_switch() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "rt_flow").await;
    let root = std::env::temp_dir().join(format!("akl-runtime-{}", Uuid::new_v4()));
    let lc = Lifecycle::new(db, IdentityStore::open(&root).unwrap(), false);
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let cfg = EmulationConfig {
        discovery_bind: free_udp(),
        adoption_bind: free_tcp(),
        controller: Some(ControllerTrust {
            ip: sim.addr.ip(),
            port: sim.addr.port(),
            fingerprint: sim.fingerprint,
        }),
        control: ControlConfig {
            keepalive_interval: Duration::from_millis(100),
            timeout: Duration::from_secs(1),
            retry_delay: Duration::from_millis(100),
            offline_after: 3,
        },
        adoption_timeout: TIMEOUT,
        poll_interval: Duration::from_millis(100),
    };
    let supervisor = tokio::spawn(supervise(lc.clone(), cfg.clone()));

    // Profil erst nach dem Start anlegen: der Supervisor muss es selbst finden.
    let (profile, device_pin) = lc.create_profile(AiKey, "Key").await.unwrap();
    probe_until_answered(cfg.discovery_bind).await;
    wait_for(&lc, profile.id, Discovered).await;

    let reply = send_line_pinned(
        cfg.adoption_bind,
        device_pin,
        &wire::adopt_request("rt-token"),
        TIMEOUT,
    )
    .await
    .unwrap()
    .unwrap();
    let reply: wire::Reply = serde_json::from_str(&reply).unwrap();
    assert!(reply.ok, "{reply:?}");
    wait_for(&lc, profile.id, Online).await;

    // Kill-Switch: Profil deaktiviert, Dienste beendet – keine Antwort mehr.
    lc.set_kill_switch(true, "tester", None).await.unwrap();
    assert_eq!(state(&lc, profile.id).await, Disabled);
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(
        discovery_probe(
            cfg.discovery_bind,
            &[1, 0, 0, 0],
            Duration::from_millis(300)
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(
        send_line_pinned(
            cfg.adoption_bind,
            device_pin,
            &wire::adopt_request("x"),
            Duration::from_millis(500)
        )
        .await
        .is_err(),
        "Adoptions-Endpunkt muss geschlossen sein"
    );
    supervisor.abort();
    let _ = std::fs::remove_dir_all(root);
}
