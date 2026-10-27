//! Lifecycle-Service gegen eine echte PostgreSQL-Instanz.

use std::path::PathBuf;

use akl_core::{DeviceProfileKind::*, DeviceState::*};
use akl_device_lifecycle::{
    identity::IdentityStore,
    service::{Lifecycle, LifecycleError},
};
use akl_persistence::{
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
        correlation_id: None,
    }
}

async fn lifecycle(schema: &str, forced: bool) -> Option<(Lifecycle, PathBuf)> {
    let url = test_database_url()?;
    let db = fresh_schema(&url, schema).await;
    let root = std::env::temp_dir().join(format!("akl-lifecycle-{}", Uuid::new_v4()));
    let store = IdentityStore::open(&root).unwrap();
    Some((Lifecycle::new(db, store, forced), root))
}

fn identity_dirs(root: &PathBuf) -> usize {
    std::fs::read_dir(root).unwrap().count()
}

#[tokio::test]
async fn profile_gets_identity_and_rejected_profile_leaves_none() {
    let Some((lc, root)) = lifecycle("lc_create", false).await else {
        return;
    };
    let (profile, fingerprint) = lc.create_profile(AiKey, "Key").await.unwrap();
    assert_eq!(profile.state, Unconfigured);
    assert_eq!(lc.fingerprint(&profile).unwrap(), Some(fingerprint));
    assert_eq!(identity_dirs(&root), 1);

    // Zweites aktives ai_key: abgelehnt, Identität zurückgerollt.
    assert!(matches!(
        lc.create_profile(AiKey, "Key 2").await.unwrap_err(),
        LifecycleError::ActiveAiKeyExists
    ));
    assert_eq!(identity_dirs(&root), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn kill_switch_blocks_creation_and_reactivation() {
    let Some((lc, root)) = lifecycle("lc_kill", false).await else {
        return;
    };
    let (profile, _) = lc.create_profile(AiPort, "Port").await.unwrap();
    let disabled = lc.set_kill_switch(true, "admin", None).await.unwrap();
    assert_eq!(disabled.len(), 1);
    assert!(lc.kill_switch().await.unwrap().active);
    assert!(matches!(
        lc.create_profile(AiKey, "Key").await.unwrap_err(),
        LifecycleError::KillSwitchActive
    ));
    assert!(matches!(
        lc.transition(profile.id, to(Unconfigured))
            .await
            .unwrap_err(),
        LifecycleError::Transition(TransitionError::KillSwitch)
    ));
    lc.set_kill_switch(false, "admin", None).await.unwrap();
    lc.transition(profile.id, to(Unconfigured)).await.unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn forced_kill_switch_cannot_be_released() {
    let Some((lc, root)) = lifecycle("lc_forced", false).await else {
        return;
    };
    let (profile, _) = lc.create_profile(AiKey, "Key").await.unwrap();
    // Neustart mit AKL_DEVICE_KILL_SWITCH=true auf derselben Datenbank.
    let forced = Lifecycle::new(lc.db().clone(), IdentityStore::open(&root).unwrap(), true);
    let disabled = forced.enforce_forced_kill_switch().await.unwrap();
    assert_eq!(disabled.len(), 1);
    let status = forced.kill_switch().await.unwrap();
    assert!(status.active && status.forced);
    assert!(matches!(
        forced
            .set_kill_switch(false, "admin", None)
            .await
            .unwrap_err(),
        LifecycleError::KillSwitchForced
    ));
    assert!(
        forced
            .transition(profile.id, to(Unconfigured))
            .await
            .is_err()
    );
    // Deaktivieren bleibt erlaubt (hier: bereits deaktiviert → unzulässig, aber kein Kill-Switch-Fehler).
    assert!(matches!(
        forced
            .transition(profile.id, to(Disabled))
            .await
            .unwrap_err(),
        LifecycleError::Transition(TransitionError::Invalid(_))
    ));
    std::fs::remove_dir_all(root).unwrap();
}
