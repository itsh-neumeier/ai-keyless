//! Pinning-Tests gegen den simulierten Controller (synthetic_verified).

use std::time::Duration;

use akl_protect_compat::tls::{ConnectError, Fingerprint, connect_pinned, inspect_fingerprint};
use akl_protect_sim::{Behavior, SimController};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const T: Duration = Duration::from_secs(2);

#[tokio::test]
async fn inspect_reports_the_servers_fingerprint_without_sending_data() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let fp = inspect_fingerprint("127.0.0.1", sim.addr.port(), T)
        .await
        .unwrap();
    assert_eq!(fp, sim.fingerprint);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(sim.received().is_empty());
}

#[tokio::test]
async fn credentials_flow_only_over_the_pinned_connection() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let mut tls = connect_pinned("127.0.0.1", sim.addr.port(), sim.fingerprint, T)
        .await
        .unwrap();
    tls.write_all(b"credential=synthetisch\n").await.unwrap();
    let mut line = String::new();
    BufReader::new(&mut tls).read_line(&mut line).await.unwrap();
    assert_eq!(line, "OK\n");
    assert_eq!(sim.received(), vec!["credential=synthetisch"]);
}

#[tokio::test]
async fn wrong_pin_fails_before_any_data_is_sent() {
    let sim = SimController::start(Behavior::Normal).await.unwrap();
    let wrong = Fingerprint::of(b"anderes zertifikat");
    let err = connect_pinned("127.0.0.1", sim.addr.port(), wrong, T)
        .await
        .unwrap_err();
    assert!(err.is_pin_mismatch(), "{err}");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        sim.received().is_empty(),
        "kein Byte Nutzdaten an einen fremden Server"
    );
}

#[tokio::test]
async fn certificate_change_is_detected() {
    // Ein neu aufgesetzter (oder untergeschobener) Controller hat ein anderes
    // Zertifikat – der alte Pin darf ihn nicht akzeptieren.
    let original = SimController::start(Behavior::Normal).await.unwrap();
    let replacement = SimController::start(Behavior::Normal).await.unwrap();
    assert_ne!(original.fingerprint, replacement.fingerprint);
    let err = connect_pinned(
        "127.0.0.1",
        replacement.addr.port(),
        original.fingerprint,
        T,
    )
    .await
    .unwrap_err();
    assert!(err.is_pin_mismatch());
}

#[tokio::test]
async fn silent_and_resetting_servers_fail_cleanly() {
    let silent = SimController::start(Behavior::Silent).await.unwrap();
    let err = connect_pinned(
        "127.0.0.1",
        silent.addr.port(),
        silent.fingerprint,
        Duration::from_millis(300),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ConnectError::Timeout), "{err}");

    let reset = SimController::start(Behavior::Reset).await.unwrap();
    let err = connect_pinned("127.0.0.1", reset.addr.port(), reset.fingerprint, T)
        .await
        .unwrap_err();
    assert!(
        matches!(err, ConnectError::Io(_)) && !err.is_pin_mismatch(),
        "{err}"
    );
}
