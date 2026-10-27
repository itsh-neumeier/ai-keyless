//! Geräteprofile, Übergänge und Kill-Switch gegen eine echte PostgreSQL-Instanz.

use akl_core::{DeviceProfileKind::*, DeviceState::*};
use akl_persistence::{
    Database,
    devices::{TransitionError, TransitionRequest},
    testing::{fresh_schema, test_database_url},
};
use uuid::Uuid;

fn to(state: akl_core::DeviceState) -> TransitionRequest<'static> {
    TransitionRequest {
        to: state,
        expected_from: None,
        reason: "test",
        actor: "tester",
        correlation_id: Some("req-1"),
    }
}

async fn db(schema: &str) -> Option<Database> {
    let url = test_database_url()?;
    Some(fresh_schema(&url, schema).await)
}

#[tokio::test]
async fn transitions_are_checked_and_logged() {
    let Some(db) = db("dev_transitions").await else {
        return;
    };
    let id = Uuid::new_v4();
    let p = db
        .create_device_profile(id, AiKey, "Test-Key")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(p.state, Unconfigured);

    db.transition_device(id, to(Discovered)).await.unwrap();
    // Unzulässig: Adoption überspringen. Zustand bleibt, kein Log-Eintrag.
    let err = db.transition_device(id, to(Online)).await.unwrap_err();
    assert!(matches!(err, TransitionError::Invalid(_)), "{err:?}");
    assert_eq!(
        db.get_device_profile(id).await.unwrap().unwrap().state,
        Discovered
    );

    // Optimistische Prüfung schlägt bei veraltetem Ausgangszustand fehl.
    let stale = TransitionRequest {
        expected_from: Some(Unconfigured),
        ..to(PendingAdoption)
    };
    assert!(matches!(
        db.transition_device(id, stale).await.unwrap_err(),
        TransitionError::Stale {
            expected: Unconfigured,
            actual: Discovered
        }
    ));

    let ok = TransitionRequest {
        expected_from: Some(Discovered),
        ..to(PendingAdoption)
    };
    assert_eq!(
        db.transition_device(id, ok).await.unwrap().state,
        PendingAdoption
    );

    let log = db.device_transitions(id, 10).await.unwrap();
    let steps: Vec<_> = log
        .iter()
        .map(|t| (t.from_state.as_str(), t.to_state.as_str()))
        .collect();
    assert_eq!(
        steps,
        [
            ("discovered", "pending_adoption"),
            ("unconfigured", "discovered")
        ]
    );
    assert!(log.iter().all(|t| t.actor == "tester"
        && t.reason.as_deref() == Some("test")
        && t.correlation_id.as_deref() == Some("req-1")));

    assert!(matches!(
        db.transition_device(Uuid::new_v4(), to(Disabled))
            .await
            .unwrap_err(),
        TransitionError::NotFound
    ));
}

#[tokio::test]
async fn only_one_active_ai_key() {
    let Some(db) = db("dev_single_key").await else {
        return;
    };
    let first = Uuid::new_v4();
    db.create_device_profile(first, AiKey, "A")
        .await
        .unwrap()
        .unwrap();
    assert!(
        db.create_device_profile(Uuid::new_v4(), AiKey, "B")
            .await
            .unwrap()
            .is_none()
    );
    // ai_port ist ein eigenes Profil (ADR-003) und davon unabhängig.
    assert!(
        db.create_device_profile(Uuid::new_v4(), AiPort, "Port")
            .await
            .unwrap()
            .is_some()
    );

    // Erstes deaktivieren → zweites ai_key möglich; Reaktivieren des ersten scheitert.
    db.transition_device(first, to(Disabled)).await.unwrap();
    db.create_device_profile(Uuid::new_v4(), AiKey, "B")
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        db.transition_device(first, to(Unconfigured))
            .await
            .unwrap_err(),
        TransitionError::ActiveAiKeyExists
    ));
    assert_eq!(
        db.get_device_profile(first).await.unwrap().unwrap().state,
        Disabled
    );
}

#[tokio::test]
async fn kill_switch_disables_everything_and_blocks_transitions() {
    let Some(db) = db("dev_kill_switch").await else {
        return;
    };
    let key = Uuid::new_v4();
    let port = Uuid::new_v4();
    db.create_device_profile(key, AiKey, "Key")
        .await
        .unwrap()
        .unwrap();
    db.create_device_profile(port, AiPort, "Port")
        .await
        .unwrap()
        .unwrap();
    db.transition_device(key, to(Discovered)).await.unwrap();
    assert!(!db.kill_switch().await.unwrap().active);

    let disabled = db.set_kill_switch(true, "admin", None).await.unwrap();
    assert_eq!(disabled.len(), 2);
    assert!(disabled.iter().all(|p| p.state == Disabled));
    let ks = db.kill_switch().await.unwrap();
    assert!(ks.active);
    assert_eq!(ks.changed_by.as_deref(), Some("admin"));
    let last = &db.device_transitions(key, 1).await.unwrap()[0];
    assert_eq!(
        (last.from_state.as_str(), last.reason.as_deref()),
        ("discovered", Some("kill_switch"))
    );

    // Solange aktiv: kein Reaktivieren.
    assert!(matches!(
        db.transition_device(key, to(Unconfigured))
            .await
            .unwrap_err(),
        TransitionError::KillSwitch
    ));
    // Erneutes Einschalten deaktiviert nichts Weiteres.
    assert!(
        db.set_kill_switch(true, "admin", None)
            .await
            .unwrap()
            .is_empty()
    );

    assert!(
        db.set_kill_switch(false, "admin", None)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.transition_device(key, to(Unconfigured))
            .await
            .unwrap()
            .state,
        Unconfigured
    );
}
