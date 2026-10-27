//! Gemeinsamer Test-Client für API-Integrationstests (Cookie + CSRF).
#![allow(dead_code)]

use akl_api::{AppState, app, crypto};
use akl_persistence::Database;
use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

pub const ADMIN_PW: &str = "korrekt-pferd-batterie";

pub struct Client {
    pub router: Router,
    pub cookie: Option<String>,
    pub csrf: Option<String>,
}

pub struct Resp {
    pub status: u16,
    pub body: Value,
    pub set_cookie: Option<String>,
}

impl Client {
    pub fn new(db: Database) -> Self {
        Self::with_state(AppState::new(Some(db), false))
    }

    pub fn with_state(state: AppState) -> Self {
        Self {
            router: app(state, None),
            cookie: None,
            csrf: None,
        }
    }

    pub async fn send(&mut self, method: &str, uri: &str, body: Option<Value>) -> Resp {
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
    pub fn adopt(&mut self, r: &Resp) {
        let raw = r.set_cookie.as_ref().expect("Set-Cookie");
        self.cookie = Some(raw.split(';').next().unwrap().to_owned());
        self.csrf = r.body["csrf_token"].as_str().map(str::to_owned);
    }
}

pub async fn issue_token(db: &Database) -> String {
    let token = crypto::random_token();
    db.replace_setup_token(
        &crypto::token_hash(&token),
        Utc::now() + Duration::minutes(5),
    )
    .await
    .unwrap();
    token
}

pub async fn setup_admin(db: &Database, c: &mut Client) {
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
