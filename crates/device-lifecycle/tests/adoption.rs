//! Adoption, Keepalive und Reconnect gegen `akl-protect-sim` und eine echte
//! PostgreSQL (AKL-M040-04, synthetisch – keine Aussage über Protect).

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use akl_core::{DeviceProfileKind::*, DeviceState, DeviceState::*};
use akl_device_lifecycle::{
    adoption::{AdoptionServer, ControllerTrust, Refusal, wire},
    control::{self, ControlConfig, Exit, RunError},
    identity::IdentityStore,
    service::Lifecycle,
};
use akl_persistence::{
    devices::TransitionRequest,
    testing::{fresh_schema, test_database_url},
};
use akl_protect_compat::tls::Fingerprint;
use akl_protect_sim::{Behavior, SimCert, SimController, send_line_pinned};
use tokio::task::JoinHandle;
use uuid::Uuid;

const TIMEOUT: Duration = Duration::from_secs(2);
const TOKEN: &str = "synthetic-token-7f3a";

const FAST: ControlConfig = ControlConfig {
    keepalive_interval: Duration::from_millis(100),
    timeout: Duration::from_secs(1),
    retry_delay: Duration::from_millis(100),
    offline_after: 3,
};

struct Setup {
    lc: Lifecycle,
    profile: Uuid,
    device_pin: Fingerprint,
    adoption: SocketAddr,
    task: JoinHandle<()>,
    root: PathBuf,
}

impl Drop for Setup {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn req(to: DeviceState) -> TransitionRequest<'static> {
    TransitionRequest {
        to,
        expected_from: None,
        reason: "test",
        actor: "tester",
        correlation_id: None,
    }
}

fn trust_for(sim: &SimController) -> ControllerTrust {
    ControllerTrust {
        ip: sim.addr.ip(),
        port: sim.addr.port(),
        fingerprint: sim.fingerprint,
    }
}

/// Profil im Zustand `discovered` mit laufendem Adoptions-Endpunkt.
async fn setup(schema: &str, trust: ControllerTrust) -> Option<Setup> {
    let url = test_database_url()?;
    let db = fresh_schema(&url, schema).await;
    let root = std::env::temp_dir().join(format!("akl-adoption-{}", Uuid::new_v4()));
    let lc = Lifecycle::new(db, IdentityStore::open(&root).unwrap(), false);
    let (profile, device_pin) = lc.create_profile(AiKey, "Key").await.unwrap();
    lc.transition(profile.id, req(Discovered)).await.unwrap();
    let server = AdoptionServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        lc.clone(),
        profile.id,
        trust,
        TIMEOUT,
    )
    .await
    .unwrap();
    let adoption = server.local_addr().unwrap();
    let task = tokio::spawn(async move {
        server.run().await.expect("Endpunkt läuft");
    });
    Some(Setup {
        lc,
        profile: profile.id,
        device_pin,
        adoption,
        task,
        root,
    })
}

async fn adopt(s: &Setup, line: &str) -> Option<wire::Reply> {
    send_line_pinned(s.adoption, s.device_pin, line, TIMEOUT)
        .await
        .ok()
        .flatten()
        .map(|reply| serde_json::from_str(&reply).expect("Antwort ist JSON"))
}

async fn state(lc: &Lifecycle, id: Uuid) -> DeviceState {
    lc.db().get_device_profile(id).await.unwrap().unwrap().state
}

async fn wait_for(lc: &Lifecycle, id: Uuid, target: DeviceState) {
    for _ in 0..100 {
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

fn spawn_control(lc: &Lifecycle, id: Uuid) -> JoinHandle<Result<Exit, RunError>> {
    let lc = lc.clone();
    tokio::spawn(async move { control::run(&lc, id, FAST).await })
}

/// Startet einen Simulator mit gegebenem Zertifikat auf einer festen Adresse
/// (der alte Listener kann noch kurz belegt sein).
async fn restart_sim(addr: SocketAddr, cert: SimCert) -> SimController {
    for _ in 0..100 {
        if let Ok(sim) =
            SimController::start_with(&addr.to_string(), Behavior::Normal, cert.clone()).await
        {
            return sim;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("Simulator-Neustart auf {addr} fehlgeschlagen");
}

fn hellos(sim: &SimController) -> usize {
    sim.received()
        .iter()
        .filter(|l| l.contains("\"hello\""))
        .count()
}

#[tokio::test]
async fn adoption_keepalive_restart_and_way_back() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let Some(s) = setup("adopt_flow", trust_for(&sim)).await else {
        return;
    };

    let reply = adopt(&s, &wire::adopt_request(TOKEN))
        .await
        .expect("Antwort");
    assert_eq!(
        reply,
        wire::Reply {
            ok: true,
            error: None
        }
    );
    assert_eq!(state(&s.lc, s.profile).await, Adopted);

    // Token ging nur über die gepinnte Rückverbindung und liegt nur in der Secret-Datei.
    let received = sim.received();
    assert_eq!(received.len(), 1);
    assert!(received[0].contains(TOKEN));
    let binding = s.lc.identities().load_binding(s.profile).unwrap().unwrap();
    assert_eq!(binding.trust, trust_for(&sim));
    assert!(!format!("{binding:?}").contains(TOKEN));
    let log = s.lc.db().device_transitions(s.profile, 50).await.unwrap();
    assert!(!format!("{log:?}").contains(TOKEN));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = s.root.join(s.profile.to_string()).join("controller.token");
        let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    // Kontrollkanal: adopted → online, Keepalives werden bestätigt.
    let control = spawn_control(&s.lc, s.profile);
    wait_for(&s.lc, s.profile, Online).await;
    tokio::time::sleep(FAST.keepalive_interval * 3).await;
    assert!(sim.received().iter().any(|l| l.contains("\"keepalive\"")));

    // Neustart des Dienstes: neue Lifecycle-Instanz auf denselben Daten,
    // online ohne erneute Adoption (Readback über reconnecting).
    control.abort();
    let restarted = Lifecycle::new(
        s.lc.db().clone(),
        IdentityStore::open(&s.root).unwrap(),
        false,
    );
    assert_eq!(state(&restarted, s.profile).await, Online);
    let before = hellos(&sim);
    let control = spawn_control(&restarted, s.profile);
    for _ in 0..100 {
        if hellos(&sim) > before {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    wait_for(&restarted, s.profile, Online).await;
    let log = restarted
        .db()
        .device_transitions(s.profile, 3)
        .await
        .unwrap();
    assert_eq!(
        (log[1].to_state.as_str(), log[0].to_state.as_str()),
        ("reconnecting", "online")
    );
    assert_eq!(log[0].actor, "system:control");

    // Rückweg: deaktivieren beendet den Kanal; Bindung lässt sich entfernen.
    restarted
        .transition(s.profile, req(Disabled))
        .await
        .unwrap();
    let exit = tokio::time::timeout(TIMEOUT, control)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(exit.unwrap(), Exit::Stopped);
    restarted.identities().forget_binding(s.profile).unwrap();
    assert!(
        restarted
            .identities()
            .load_binding(s.profile)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        control::run(&restarted, s.profile, FAST).await,
        Err(RunError::NotAdopted)
    ));
}

#[tokio::test]
async fn lost_controller_goes_reconnecting_offline_and_back_online() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let (addr, cert) = (sim.addr, sim.cert());
    let Some(s) = setup("adopt_reconnect", trust_for(&sim)).await else {
        return;
    };
    assert!(adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap().ok);
    let control = spawn_control(&s.lc, s.profile);
    wait_for(&s.lc, s.profile, Online).await;

    drop(sim);
    wait_for(&s.lc, s.profile, Offline).await;
    let log = s.lc.db().device_transitions(s.profile, 10).await.unwrap();
    assert!(log.iter().any(|t| t.to_state == "reconnecting"));

    let sim = restart_sim(addr, cert).await;
    wait_for(&s.lc, s.profile, Online).await;
    assert!(hellos(&sim) >= 1);
    control.abort();
}

#[tokio::test]
async fn wrong_controller_pin_keeps_token_and_rolls_back() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let trust = ControllerTrust {
        fingerprint: Fingerprint::of(b"anderes-zertifikat"),
        ..trust_for(&sim)
    };
    let Some(s) = setup("adopt_pin", trust).await else {
        return;
    };
    let reply = adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap();
    assert_eq!(reply.error, Some(Refusal::ControllerUntrusted));
    assert_eq!(state(&s.lc, s.profile).await, Discovered);
    assert!(
        sim.received().is_empty(),
        "Token darf nicht gesendet werden"
    );
    assert!(s.lc.identities().load_binding(s.profile).unwrap().is_none());
    let log = s.lc.db().device_transitions(s.profile, 2).await.unwrap();
    assert_eq!(
        (log[1].to_state.as_str(), log[0].to_state.as_str()),
        ("pending_adoption", "discovered")
    );
}

#[tokio::test]
async fn unreachable_controller_is_refused() {
    let sim = SimController::start(Behavior::Reset).await.unwrap();
    let Some(s) = setup("adopt_unreach", trust_for(&sim)).await else {
        return;
    };
    let reply = adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap();
    assert_eq!(reply.error, Some(Refusal::ControllerUnreachable));
    assert_eq!(state(&s.lc, s.profile).await, Discovered);
    assert!(s.lc.identities().load_binding(s.profile).unwrap().is_none());
}

#[tokio::test]
async fn invalid_requests_wrong_state_and_kill_switch() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let Some(s) = setup("adopt_refuse", trust_for(&sim)).await else {
        return;
    };
    for bad in [
        "kein json",
        r#"{"type":"adopt"}"#,
        r#"{"type":"adopt","token":""}"#,
        r#"{"type":"adopt","token":"a","extra":1}"#,
        r#"{"type":"steal","token":"a"}"#,
    ] {
        let reply = adopt(&s, bad).await.unwrap();
        assert_eq!(reply.error, Some(Refusal::BadRequest), "{bad}");
    }
    let long = wire::adopt_request(&"x".repeat(2000));
    assert_eq!(
        adopt(&s, &long).await.unwrap().error,
        Some(Refusal::BadRequest)
    );
    assert_eq!(state(&s.lc, s.profile).await, Discovered);

    s.lc.set_kill_switch(true, "tester", None).await.unwrap();
    let reply = adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap();
    assert_eq!(reply.error, Some(Refusal::Disabled));
    s.lc.set_kill_switch(false, "tester", None).await.unwrap();
    // Deaktiviertes Profil bleibt abgelehnt, ebenso der falsche Zustand.
    assert_eq!(
        adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap().error,
        Some(Refusal::Disabled)
    );
    s.lc.transition(s.profile, req(Unconfigured)).await.unwrap();
    assert_eq!(
        adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap().error,
        Some(Refusal::WrongState)
    );
    assert!(sim.received().is_empty());
}

#[tokio::test]
async fn foreign_source_gets_no_handshake() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    // Vertrauter Controller ist 127.0.0.2; die Anfrage kommt von 127.0.0.1.
    let trust = ControllerTrust {
        ip: "127.0.0.2".parse().unwrap(),
        ..trust_for(&sim)
    };
    let Some(s) = setup("adopt_foreign", trust).await else {
        return;
    };
    assert!(adopt(&s, &wire::adopt_request(TOKEN)).await.is_none());
    assert_eq!(state(&s.lc, s.profile).await, Discovered);
}

#[tokio::test]
async fn changed_controller_certificate_stops_channel_with_error() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let addr = sim.addr;
    let Some(s) = setup("adopt_certchange", trust_for(&sim)).await else {
        return;
    };
    assert!(adopt(&s, &wire::adopt_request(TOKEN)).await.unwrap().ok);
    drop(sim);

    // Gleiche Adresse, anderes Zertifikat (z. B. untergeschobener Controller).
    let imposter = restart_sim(addr, SimCert::generate().unwrap()).await;
    let exit = tokio::time::timeout(TIMEOUT * 3, control::run(&s.lc, s.profile, FAST))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(exit, Exit::PinMismatch);
    assert_eq!(state(&s.lc, s.profile).await, Error);
    assert!(imposter.received().is_empty());
}
