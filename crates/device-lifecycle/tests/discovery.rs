//! Discovery-Antwort gegen `akl-protect-sim` und eine echte PostgreSQL
//! (AKL-M040-03, synthetisch – keine Aussage über Protect).

use std::{path::PathBuf, time::Duration};

use akl_core::{DeviceProfileKind::*, DeviceState::*};
use akl_device_lifecycle::{
    discovery::{Announcement, Responder},
    identity::IdentityStore,
    service::Lifecycle,
};
use akl_persistence::{
    devices::TransitionRequest,
    testing::{fresh_schema, test_database_url},
};
use akl_protect_compat::discovery::{Packet, tlv};
use akl_protect_sim::discovery_probe;
use tokio::task::JoinHandle;
use uuid::Uuid;

const WAIT: Duration = Duration::from_millis(500);
const SILENCE: Duration = Duration::from_millis(300);

struct Setup {
    lc: Lifecycle,
    profile: Uuid,
    addr: std::net::SocketAddr,
    task: JoinHandle<()>,
    root: PathBuf,
}

impl Drop for Setup {
    fn drop(&mut self) {
        self.task.abort();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

async fn setup(schema: &str) -> Option<Setup> {
    let url = test_database_url()?;
    let db = fresh_schema(&url, schema).await;
    let root = std::env::temp_dir().join(format!("akl-discovery-{}", Uuid::new_v4()));
    let lc = Lifecycle::new(db, IdentityStore::open(&root).unwrap(), false);
    let (profile, _) = lc.create_profile(AiKey, "Key").await.unwrap();
    let responder = Responder::bind(
        "127.0.0.1:0".parse().unwrap(),
        lc.clone(),
        profile.id,
        Announcement::neutral(profile.id, AiKey),
    )
    .await
    .unwrap();
    let addr = responder.local_addr().unwrap();
    let task = tokio::spawn(async move {
        responder.run().await.expect("Responder läuft");
    });
    Some(Setup {
        lc,
        profile: profile.id,
        addr,
        task,
        root,
    })
}

async fn state(s: &Setup) -> akl_core::DeviceState {
    s.lc.db()
        .get_device_profile(s.profile)
        .await
        .unwrap()
        .unwrap()
        .state
}

#[tokio::test]
async fn probe_is_answered_and_marks_profile_discovered() {
    let Some(s) = setup("disc_answer").await else {
        return;
    };
    assert_eq!(state(&s).await, Unconfigured);

    let bytes = discovery_probe(s.addr, &[1, 0, 0, 0], WAIT)
        .await
        .unwrap()
        .expect("Antwort auf v1-Probe");
    let reply = Packet::decode(&bytes).unwrap();
    let expected = Announcement::neutral(s.profile, AiKey);
    assert_eq!((reply.version, reply.command), (1, 0));
    assert_eq!(
        reply.find(tlv::HOSTNAME),
        Some(expected.hostname.as_bytes())
    );
    assert_eq!(reply.find(tlv::PRODUCT), Some(expected.product.as_bytes()));
    assert_eq!(reply.find(tlv::MODEL_V1), Some(expected.model.as_bytes()));
    assert_eq!(reply.find(tlv::VERSION), Some(expected.version.as_bytes()));

    assert_eq!(state(&s).await, Discovered);
    let log = s.lc.db().device_transitions(s.profile, 10).await.unwrap();
    assert_eq!(log[0].to_state, "discovered");
    assert_eq!(log[0].actor, "system:discovery");

    // Weitere Anfragen (nach der Sperrfrist) ändern den Zustand nicht erneut.
    tokio::time::sleep(akl_device_lifecycle::discovery::MIN_REPLY_INTERVAL).await;
    assert!(
        discovery_probe(s.addr, &[2, 8, 0, 0], WAIT)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(state(&s).await, Discovered);
    assert_eq!(
        s.lc.db()
            .device_transitions(s.profile, 10)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn malformed_or_foreign_packets_get_no_reply_and_no_transition() {
    let Some(s) = setup("disc_junk").await else {
        return;
    };
    for junk in [
        &[][..],
        &[1, 0, 0],
        &[1, 0, 0, 5, 1],
        &[2, 6, 0, 0],
        &[1, 0, 0, 3, 0x0b, 0, 0],
        b"hello",
    ] {
        assert!(
            discovery_probe(s.addr, junk, SILENCE)
                .await
                .unwrap()
                .is_none(),
            "{junk:?}"
        );
    }
    assert_eq!(state(&s).await, Unconfigured);
    assert!(
        s.lc.db()
            .device_transitions(s.profile, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn repeated_probes_are_rate_limited() {
    let Some(s) = setup("disc_rate").await else {
        return;
    };
    assert!(
        discovery_probe(s.addr, &[1, 0, 0, 0], WAIT)
            .await
            .unwrap()
            .is_some()
    );
    // Gleicher Absender (127.0.0.1) innerhalb der Sperrfrist: keine Antwort.
    assert!(
        discovery_probe(s.addr, &[1, 0, 0, 0], SILENCE)
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn kill_switch_and_disabled_profile_silence_the_responder() {
    let Some(s) = setup("disc_kill").await else {
        return;
    };
    s.lc.set_kill_switch(true, "tester", None).await.unwrap();
    assert!(
        discovery_probe(s.addr, &[1, 0, 0, 0], SILENCE)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(state(&s).await, Disabled);

    // Kill-Switch aus, Profil bleibt deaktiviert: weiterhin keine Antwort.
    s.lc.set_kill_switch(false, "tester", None).await.unwrap();
    assert!(
        discovery_probe(s.addr, &[1, 0, 0, 0], SILENCE)
            .await
            .unwrap()
            .is_none()
    );

    // Erst nach bewusster Reaktivierung antwortet das Gerät wieder.
    s.lc.transition(
        s.profile,
        TransitionRequest {
            to: Unconfigured,
            expected_from: Some(Disabled),
            reason: "test",
            actor: "tester",
            correlation_id: None,
        },
    )
    .await
    .unwrap();
    assert!(
        discovery_probe(s.addr, &[1, 0, 0, 0], WAIT)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(state(&s).await, Discovered);
}
