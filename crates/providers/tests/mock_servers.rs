//! Provider-Tests gegen lokale Mock-Server (synthetisch, keine echten Modelle).

use std::time::Duration;

use akl_providers::{
    NetworkPolicy, Provider, ProviderConfig, ProviderKind, SYNTHETIC_TEST_IMAGE_PNG,
    SYNTHETIC_TEST_PROMPT, synthetic_answer_matches,
};
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};

async fn serve(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://127.0.0.1:{}", addr.port())
}

fn loopback() -> NetworkPolicy {
    NetworkPolicy {
        allow_loopback: true,
        ..NetworkPolicy::default()
    }
}

fn provider(kind: ProviderKind, base_url: String, api_key: Option<&str>) -> Provider {
    Provider::new(
        ProviderConfig {
            kind,
            base_url,
            api_key: api_key.map(str::to_owned),
            timeout: Duration::from_secs(2),
        },
        loopback(),
    )
}

#[tokio::test]
async fn ollama_models_and_vision() {
    let router = Router::new()
        .route(
            "/api/tags",
            get(|| async {
                Json(json!({ "models": [{ "name": "qwen2.5vl:3b" }, { "name": "llava:7b" }] }))
            }),
        )
        .route(
            "/api/chat",
            post(|Json(body): Json<Value>| async move {
                // Prüft das Anfrageformat, wie Ollama es erwartet.
                assert_eq!(body["stream"], false);
                assert_eq!(body["model"], "qwen2.5vl:3b");
                assert!(body["messages"][0]["images"][0].as_str().unwrap().len() > 100);
                Json(json!({ "message": { "role": "assistant", "content": " Red. " } }))
            }),
        );
    let p = provider(ProviderKind::Ollama, serve(router).await, None);

    let models = p.list_models().await.unwrap();
    assert_eq!(models.models, vec!["llava:7b", "qwen2.5vl:3b"]);

    let r = p
        .describe_image(
            "qwen2.5vl:3b",
            SYNTHETIC_TEST_PROMPT,
            SYNTHETIC_TEST_IMAGE_PNG,
        )
        .await
        .unwrap();
    assert_eq!(r.text, "Red.");
    assert!(synthetic_answer_matches(&r.text));
}

#[tokio::test]
async fn openai_compatible_with_bearer_and_base_path() {
    let router = Router::new()
        .route(
            "/v1/models",
            get(|headers: HeaderMap| async move {
                if headers
                    .get(header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    != Some("Bearer geheim")
                {
                    return StatusCode::UNAUTHORIZED.into_response();
                }
                Json(json!({ "data": [{ "id": "llava-v1.6" }] })).into_response()
            }),
        )
        .route(
            "/v1/chat/completions",
            post(|Json(body): Json<Value>| async move {
                let url = body["messages"][0]["content"][1]["image_url"]["url"]
                    .as_str()
                    .unwrap();
                assert!(url.starts_with("data:image/png;base64,"));
                Json(json!({ "choices": [{ "message": { "content": "red" } }] }))
            }),
        );
    let base = format!("{}/v1", serve(router).await);

    let ok = provider(ProviderKind::OpenaiCompatible, base.clone(), Some("geheim"));
    assert_eq!(ok.list_models().await.unwrap().models, vec!["llava-v1.6"]);
    assert_eq!(
        ok.describe_image("llava-v1.6", "x", SYNTHETIC_TEST_IMAGE_PNG)
            .await
            .unwrap()
            .text,
        "red"
    );

    let wrong = provider(ProviderKind::OpenaiCompatible, base, Some("falsch"));
    assert_eq!(
        wrong.list_models().await.unwrap_err().code(),
        "provider_auth_failed"
    );
}

#[tokio::test]
async fn redirects_are_not_followed() {
    // Eine Weiterleitung auf die Metadaten-Adresse darf nie verfolgt werden.
    let router = Router::new().route(
        "/api/tags",
        get(|| async {
            (
                StatusCode::FOUND,
                [(header::LOCATION, "http://169.254.169.254/latest/meta-data")],
            )
        }),
    );
    let p = provider(ProviderKind::Ollama, serve(router).await, None);
    assert_eq!(
        p.list_models().await.unwrap_err().code(),
        "redirect_blocked"
    );
}

#[tokio::test]
async fn oversized_and_malformed_responses_are_rejected() {
    let big = "x".repeat(3 * 1024 * 1024);
    let router = Router::new()
        .route(
            "/big/api/tags",
            get(move || {
                let big = big.clone();
                async move { big }
            }),
        )
        .route(
            "/bad/api/tags",
            get(|| async { Json(json!({ "unexpected": true })) }),
        );
    let base = serve(router).await;
    let p = provider(ProviderKind::Ollama, format!("{base}/big"), None);
    assert_eq!(
        p.list_models().await.unwrap_err().code(),
        "response_too_large"
    );
    let p = provider(ProviderKind::Ollama, format!("{base}/bad"), None);
    assert_eq!(
        p.list_models().await.unwrap_err().code(),
        "invalid_response"
    );
}

#[tokio::test]
async fn slow_provider_times_out() {
    let router = Router::new().route(
        "/api/tags",
        get(|| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Json(json!({ "models": [] }))
        }),
    );
    let p = provider(ProviderKind::Ollama, serve(router).await, None);
    assert_eq!(p.list_models().await.unwrap_err().code(), "timeout");
}

#[tokio::test]
async fn blocked_and_unreachable_targets() {
    let strict = Provider::new(
        ProviderConfig {
            kind: ProviderKind::Ollama,
            base_url: "http://127.0.0.1:9".into(),
            api_key: None,
            timeout: Duration::from_secs(2),
        },
        NetworkPolicy::default(),
    );
    assert_eq!(
        strict.list_models().await.unwrap_err().code(),
        "blocked_address"
    );

    let p = provider(ProviderKind::Ollama, "http://127.0.0.1:9".into(), None);
    assert_eq!(p.list_models().await.unwrap_err().code(), "connect_failed");
}
