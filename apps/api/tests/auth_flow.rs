//! Ende-zu-Ende-Tests der Authentifizierung gegen eine echte PostgreSQL.
//! Benötigt `AKL_TEST_DATABASE_URL` (siehe `scripts/dev-db.sh`).

mod common;

use akl_persistence::{
    Database,
    testing::{fresh_schema, test_database_url},
};
use common::{ADMIN_PW, Client, issue_token, setup_admin};
use serde_json::json;

#[tokio::test]
async fn setup_login_session_logout() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_flow").await;
    let mut c = Client::new(db.clone());

    // Vor der Einrichtung: Einrichtung nötig, Status geschützt.
    let r = c.send("GET", "/api/v1/auth/state", None).await;
    assert_eq!(r.body["setup_required"], true);
    for uri in ["/api/v1/system/status", "/api/v1/compat/profiles"] {
        assert_eq!(c.send("GET", uri, None).await.status, 401, "{uri}");
    }

    // Falsches Token, zu kurzes Passwort, ungültiger Name.
    let r = c
        .send(
            "POST",
            "/api/v1/auth/setup",
            Some(json!({"token": "falsch", "username": "admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (400, Some("invalid_setup_token"))
    );
    let token = issue_token(&db).await;
    let r = c
        .send(
            "POST",
            "/api/v1/auth/setup",
            Some(json!({"token": token, "username": "admin", "password": "kurz"})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (422, Some("invalid_password"))
    );
    let r = c
        .send(
            "POST",
            "/api/v1/auth/setup",
            Some(json!({"token": token, "username": "a b", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "invalid_username");

    // Erfolgreiche Einrichtung meldet direkt an.
    let r = c
        .send(
            "POST",
            "/api/v1/auth/setup",
            Some(json!({"token": token, "username": "Admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(r.body["user"]["username"], "admin");
    assert_eq!(r.body["user"]["role"], "admin");
    let cookie = r.set_cookie.clone().unwrap();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
    c.adopt(&r);

    // Token ist verbraucht; zweite Einrichtung unmöglich.
    let r = c
        .send(
            "POST",
            "/api/v1/auth/setup",
            Some(json!({"token": token, "username": "zwei", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.status, 409);
    assert_eq!(
        c.send("GET", "/api/v1/auth/state", None).await.body["setup_required"],
        false
    );

    // Mit Sitzung: Status lesbar, Sitzung abrufbar.
    let r = c.send("GET", "/api/v1/system/status", None).await;
    assert_eq!(r.status, 200);
    assert_eq!(r.body["database"]["status"], "ok");
    let r = c.send("GET", "/api/v1/auth/session", None).await;
    assert_eq!(r.body["user"]["username"], "admin");

    // Kompatibilitätsprofile: getrennt pro Gerät, ohne native Evidenz.
    let r = c.send("GET", "/api/v1/compat/profiles", None).await;
    assert_eq!(r.status, 200);
    let profiles = r.body["profiles"].as_array().unwrap();
    assert_eq!(profiles[0]["profile"], "ai_key");
    assert_eq!(profiles[1]["profile"], "ai_port");
    assert!(!r.body.to_string().contains("native_verified"));

    // Abmelden ohne CSRF-Token wird abgelehnt.
    let csrf = c.csrf.take();
    let r = c.send("POST", "/api/v1/auth/logout", None).await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (403, Some("csrf_failed"))
    );
    c.csrf = Some("falsch".into());
    assert_eq!(
        c.send("POST", "/api/v1/auth/logout", None).await.status,
        403
    );
    c.csrf = csrf;
    let r = c.send("POST", "/api/v1/auth/logout", None).await;
    assert_eq!(r.status, 204);
    assert!(r.set_cookie.unwrap().contains("Max-Age=0"));

    // Altes Cookie ist danach wertlos.
    assert_eq!(
        c.send("GET", "/api/v1/system/status", None).await.status,
        401
    );

    // Anmeldung: falsches Passwort, unbekanntes Konto, dann korrekt.
    c.cookie = None;
    c.csrf = None;
    for (user, pw) in [("admin", "falsch-falsch-falsch"), ("niemand", ADMIN_PW)] {
        let r = c
            .send(
                "POST",
                "/api/v1/auth/login",
                Some(json!({"username": user, "password": pw})),
            )
            .await;
        assert_eq!(
            (r.status, r.body["error"]["code"].as_str()),
            (401, Some("invalid_credentials"))
        );
    }
    let r = c
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": " ADMIN ", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.status, 200, "{}", r.body);
    c.adopt(&r);
    assert_eq!(
        c.send("GET", "/api/v1/system/status", None).await.status,
        200
    );

    let actions: Vec<String> = db
        .audit_actions()
        .await
        .unwrap()
        .into_iter()
        .map(|(_, a)| a)
        .collect();
    for expected in [
        "auth.setup_rejected",
        "auth.setup_completed",
        "auth.logout",
        "auth.login_failed",
        "auth.login",
    ] {
        assert!(
            actions.iter().any(|a| a == expected),
            "Audit fehlt: {expected} in {actions:?}"
        );
    }
}

#[tokio::test]
async fn roles_and_user_management() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_roles").await;
    let mut admin = Client::new(db.clone());
    setup_admin(&db, &mut admin).await;

    let r = admin
        .send("POST", "/api/v1/users", Some(json!({"username": "betrachter", "password": "noch-ein-langes-pw", "role": "viewer"})))
        .await;
    assert_eq!(r.status, 201, "{}", r.body);
    assert_eq!(r.body["role"], "viewer");
    let r = admin
        .send("POST", "/api/v1/users", Some(json!({"username": "betrachter", "password": "noch-ein-langes-pw", "role": "viewer"})))
        .await;
    assert_eq!(r.status, 409);
    let r = admin
        .send(
            "POST",
            "/api/v1/users",
            Some(json!({"username": "x1x", "password": "noch-ein-langes-pw", "role": "root"})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (422, Some("invalid_request")),
        "unbekannte Rolle"
    );
    let r = admin.send("GET", "/api/v1/users", None).await;
    assert_eq!(r.body.as_array().unwrap().len(), 2);
    // Passwort-Hashes verlassen nie die API.
    assert!(!r.body.to_string().contains("argon2"));

    let mut viewer = Client::new(db.clone());
    let r = viewer
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "betrachter", "password": "noch-ein-langes-pw"})),
        )
        .await;
    viewer.adopt(&r);
    assert_eq!(
        viewer
            .send("GET", "/api/v1/system/status", None)
            .await
            .status,
        200
    );
    assert_eq!(viewer.send("GET", "/api/v1/users", None).await.status, 403);
    let r = viewer
        .send(
            "POST",
            "/api/v1/users",
            Some(json!({"username": "boese", "password": "noch-ein-langes-pw", "role": "admin"})),
        )
        .await;
    assert_eq!(r.status, 403);
}

#[tokio::test]
async fn login_is_throttled_after_repeated_failures() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_throttle").await;
    let mut c = Client::new(db.clone());
    setup_admin(&db, &mut c).await;
    c.cookie = None;
    c.csrf = None;

    for _ in 0..6 {
        let r = c
            .send(
                "POST",
                "/api/v1/auth/login",
                Some(json!({"username": "admin", "password": "falsch-falsch-falsch"})),
            )
            .await;
        assert_eq!(r.status, 401);
    }
    // Auch das korrekte Passwort wird während der Sperre abgewiesen.
    let r = c
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (429, Some("too_many_attempts"))
    );
}

#[tokio::test]
async fn disabled_account_cannot_log_in() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_disabled").await;
    let mut c = Client::new(db.clone());
    setup_admin(&db, &mut c).await;
    sqlx_disable(&db, "admin").await;
    assert_eq!(
        c.send("GET", "/api/v1/system/status", None).await.status,
        401
    );
    c.cookie = None;
    let r = c
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.status, 401);
}

async fn sqlx_disable(db: &Database, username: &str) {
    db.set_user_disabled(username, true).await.unwrap();
}

#[tokio::test]
async fn disabling_accounts_and_last_admin_protection() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_disable").await;
    let mut admin = Client::new(db.clone());
    setup_admin(&db, &mut admin).await;
    let admin_id = admin.send("GET", "/api/v1/auth/session", None).await.body["user"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Sich selbst deaktivieren ist verboten.
    let r = admin
        .send(
            "PATCH",
            &format!("/api/v1/users/{admin_id}"),
            Some(json!({"disabled": true})),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (409, Some("cannot_modify_self"))
    );

    // Zweiter Admin + Operator.
    let mut ids = Vec::new();
    for (name, role) in [("admin2", "admin"), ("operator1", "operator")] {
        let r = admin
            .send(
                "POST",
                "/api/v1/users",
                Some(json!({"username": name, "password": "ein-langes-passwort", "role": role})),
            )
            .await;
        ids.push(r.body["id"].as_str().unwrap().to_owned());
    }
    let mut op = Client::new(db.clone());
    let r = op
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "operator1", "password": "ein-langes-passwort"})),
        )
        .await;
    op.adopt(&r);
    assert_eq!(
        op.send("GET", "/api/v1/system/status", None).await.status,
        200
    );

    // Operator darf keine Konten ändern.
    let r = op
        .send(
            "PATCH",
            &format!("/api/v1/users/{admin_id}"),
            Some(json!({"disabled": true})),
        )
        .await;
    assert_eq!(r.status, 403);

    // Deaktivieren beendet die Sitzung sofort; Anmeldung danach unmöglich.
    let r = admin
        .send(
            "PATCH",
            &format!("/api/v1/users/{}", ids[1]),
            Some(json!({"disabled": true})),
        )
        .await;
    assert_eq!((r.status, r.body["disabled"].as_bool()), (200, Some(true)));
    assert_eq!(
        op.send("GET", "/api/v1/system/status", None).await.status,
        401
    );
    let r = op
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "operator1", "password": "ein-langes-passwort"})),
        )
        .await;
    assert_eq!(r.status, 401);

    // Reaktivieren ermöglicht die Anmeldung wieder.
    admin
        .send(
            "PATCH",
            &format!("/api/v1/users/{}", ids[1]),
            Some(json!({"disabled": false})),
        )
        .await;
    let r = op
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "operator1", "password": "ein-langes-passwort"})),
        )
        .await;
    assert_eq!(r.status, 200);

    // Letzter aktiver Admin: admin2 deaktiviert admin; danach ist admin2 der
    // letzte aktive Admin und darf nicht deaktiviert werden.
    let mut admin2 = Client::new(db.clone());
    let r = admin2
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "admin2", "password": "ein-langes-passwort"})),
        )
        .await;
    admin2.adopt(&r);
    let r = admin2
        .send(
            "PATCH",
            &format!("/api/v1/users/{admin_id}"),
            Some(json!({"disabled": true})),
        )
        .await;
    assert_eq!(r.status, 200);
    // Direkter Repository-Test: admin2 ist jetzt der letzte aktive Admin.
    let admin2_id: uuid::Uuid = ids[0].parse().unwrap();
    assert!(matches!(
        db.set_user_disabled_by_id(admin2_id, true).await.unwrap(),
        akl_persistence::auth::DisableOutcome::LastAdmin
    ));

    let r = admin2
        .send(
            "PATCH",
            "/api/v1/users/00000000-0000-4000-8000-000000000000",
            Some(json!({"disabled": true})),
        )
        .await;
    assert_eq!(r.status, 404);
}

#[tokio::test]
async fn changing_password_requires_current_and_ends_other_sessions() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_api_password").await;
    let mut a = Client::new(db.clone());
    setup_admin(&db, &mut a).await;
    let mut b = Client::new(db.clone());
    let r = b
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "admin", "password": ADMIN_PW})),
        )
        .await;
    b.adopt(&r);

    let r = a
        .send("POST", "/api/v1/auth/password", Some(json!({"current_password": "falsch-falsch-falsch", "new_password": "neues-langes-passwort"})))
        .await;
    assert_eq!(
        (r.status, r.body["error"]["code"].as_str()),
        (400, Some("invalid_current_password"))
    );
    let r = a
        .send(
            "POST",
            "/api/v1/auth/password",
            Some(json!({"current_password": ADMIN_PW, "new_password": "kurz"})),
        )
        .await;
    assert_eq!(r.body["error"]["code"], "invalid_password");

    // Ohne CSRF-Token abgelehnt.
    let csrf = a.csrf.take();
    let r = a
        .send(
            "POST",
            "/api/v1/auth/password",
            Some(json!({"current_password": ADMIN_PW, "new_password": "neues-langes-passwort"})),
        )
        .await;
    assert_eq!(r.status, 403);
    a.csrf = csrf;

    let r = a
        .send(
            "POST",
            "/api/v1/auth/password",
            Some(json!({"current_password": ADMIN_PW, "new_password": "neues-langes-passwort"})),
        )
        .await;
    assert_eq!(r.status, 204, "{}", r.body);
    // Eigene Sitzung bleibt, die andere ist beendet.
    assert_eq!(
        a.send("GET", "/api/v1/system/status", None).await.status,
        200
    );
    assert_eq!(
        b.send("GET", "/api/v1/system/status", None).await.status,
        401
    );

    let mut c = Client::new(db.clone());
    let old = c
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(old.status, 401);
    let new = c
        .send(
            "POST",
            "/api/v1/auth/login",
            Some(json!({"username": "admin", "password": "neues-langes-passwort"})),
        )
        .await;
    assert_eq!(new.status, 200);
}
