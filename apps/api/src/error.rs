//! Einheitliche API-Fehler: `{"error": {"code": "...", "message": "..."}}`.
//! Fehlercodes sind stabil und werden in der UI übersetzt; Details landen nur im Log.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    pub fn unauthenticated() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
            "Anmeldung erforderlich",
        )
    }

    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", "Keine Berechtigung")
    }

    pub fn database_required() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "database_unavailable",
            "Datenbank nicht konfiguriert oder nicht erreichbar",
        )
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Interner Fehler",
        )
    }
}

impl From<akl_persistence::PersistenceError> for ApiError {
    fn from(err: akl_persistence::PersistenceError) -> Self {
        use akl_persistence::{PersistenceError, SqlxError};
        match err {
            PersistenceError::Database(SqlxError::PoolTimedOut | SqlxError::Io(_)) => {
                tracing::warn!(error = %err, "Datenbank nicht erreichbar");
                Self::database_required()
            }
            _ => {
                tracing::error!(error = %err, "Persistenzfehler");
                Self::internal()
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": { "code": self.code, "message": self.message } })),
        )
            .into_response()
    }
}

/// JSON-Body-Extraktor, der Fehler im einheitlichen Format meldet
/// (statt der Klartext-Antworten von `axum::Json`).
pub struct ApiJson<T>(pub T);

impl<T, S> axum::extract::FromRequest<S> for ApiJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, ApiError> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(ApiError::new(
                rejection.status(),
                "invalid_request",
                rejection.body_text(),
            )),
        }
    }
}
