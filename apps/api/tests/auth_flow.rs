//! Ende-zu-Ende-Tests der Authentifizierung gegen eine echte PostgreSQL.
//! Benötigt `AKL_TEST_DATABASE_URL` (siehe `scripts/dev-db.sh`).

use akl_api::{AppState, app, crypto};
use akl_persistence::{
    Database,
    testing::{fresh_schema, test_database_url},
};
use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const ADMIN_PW: &str = "korrekt-pferd-batterie";

struct Client {
    router: Router,
    cookie: Option<String>,
    csrf: Option<String>,
}

struct Resp {
    status: u16,
    body: Value,
    set_cookie: Option<String>,
}

impl Client {
    fn new(db: Database) -> Self {
        Self {
            router: app(AppState::new(Some(db), false), None),
            cookie: None,
            csrf: None,
        }
    }

    async fn send(&mut self, method: &str, uri: &str, body: Option<Value>) -> Resp {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "akl.test");
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        if let Some(t) = &self.csrf {
            req = req.header("x-csrf-token", t);
        }
        let req = match body {
            Some(b) => req
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status().as_u16();
        let set_cookie = res
            .headers()
            .get(header::SET_COOKIE)
            .map(|v| v.to_str().unwrap().to_owned());
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        Resp {
            status,
            body,
            set_cookie,
        }
    }

    /// Übernimmt Cookie und CSRF-Token aus einer erfolgreichen Anmeldung.
    fn adopt(&mut self, r: &Resp) {
        let raw = r.set_cookie.as_ref().expect("Set-Cookie");
        self.cookie = Some(raw.split(';').next().unwrap().to_owned());
        self.csrf = r.body["csrf_token"].as_str().map(str::to_owned);
    }
}

async fn issue_token(db: &Database) -> String {
    let token = crypto::random_token();
    db.replace_setup_token(
        &crypto::token_hash(&token),
        Utc::now() + Duration::minutes(5),
    )
    .await
    .unwrap();
    token
}

async fn setup_admin(db: &Database, c: &mut Client) {
    let token = issue_token(db).await;
    let r = c
        .send(
            "POST",
            "/api/v1/auth/setup",
            Some(json!({"token": token, "username": "Admin", "password": ADMIN_PW})),
        )
        .await;
    assert_eq!(r.status, 201, "{}", r.body);
    c.adopt(&r);
}

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
    assert_eq!(
        c.send("GET", "/api/v1/system/status", None).await.status,
        401
    );

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
