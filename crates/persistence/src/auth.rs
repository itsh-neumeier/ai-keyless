//! Repositories für Konten, Sitzungen, Einrichtungs-Token und Audit-Log.

use akl_core::Role;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{Database, PersistenceError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: Role,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub user: User,
    pub csrf_token: String,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupOutcome {
    Completed(User),
    /// Es existiert bereits ein Konto – Einrichtung ist abgeschlossen.
    AlreadyCompleted,
    /// Token unbekannt, abgelaufen oder bereits verwendet.
    InvalidToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisableOutcome {
    Updated(User),
    NotFound,
    /// Das letzte aktive Admin-Konto darf nicht deaktiviert werden.
    LastAdmin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateUserOutcome {
    Created(User),
    UsernameTaken,
}

type UserRow = (Uuid, String, String, bool, DateTime<Utc>);
/// Konto-Spalten + CSRF-Token + letzte Aktivität.
type SessionRow = (
    Uuid,
    String,
    String,
    bool,
    DateTime<Utc>,
    String,
    DateTime<Utc>,
);
/// Konto-Spalten + Passwort-Hash.
type CredentialRow = (Uuid, String, String, bool, DateTime<Utc>, String);

fn user_from_row(
    (id, username, role, disabled, created_at): UserRow,
) -> Result<User, PersistenceError> {
    let role = role
        .parse()
        .map_err(|e: akl_core::device::UnknownValue| PersistenceError::InvalidValue(e.0))?;
    Ok(User {
        id,
        username,
        role,
        disabled,
        created_at,
    })
}

pub(crate) fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .and_then(|e| e.code())
        .is_some_and(|code| code == "23505")
}

impl Database {
    pub async fn user_count(&self) -> Result<i64, PersistenceError> {
        Ok(sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(self.pool())
            .await?)
    }

    /// Ersetzt alle offenen Einrichtungs-Token durch einen neuen.
    pub async fn replace_setup_token(
        &self,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("DELETE FROM setup_tokens WHERE used_at IS NULL")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO setup_tokens (token_hash, expires_at) VALUES ($1, $2)")
            .bind(token_hash)
            .bind(expires_at)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Verbraucht das Token und legt das erste Admin-Konto atomar an.
    /// Die Tabellensperre verhindert, dass parallele Anfragen zwei Admins erzeugen.
    pub async fn complete_setup(
        &self,
        token_hash: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<SetupOutcome, PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&mut *tx)
            .await?;
        if existing > 0 {
            return Ok(SetupOutcome::AlreadyCompleted);
        }
        let consumed = sqlx::query(
            "UPDATE setup_tokens SET used_at = now() \
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()",
        )
        .bind(token_hash)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if consumed != 1 {
            return Ok(SetupOutcome::InvalidToken);
        }
        let row: UserRow = sqlx::query_as(
            "INSERT INTO users (id, username, password_hash, role) VALUES ($1, $2, $3, 'admin') \
             RETURNING id, username, role, disabled, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(username)
        .bind(password_hash)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(SetupOutcome::Completed(user_from_row(row)?))
    }

    pub async fn create_user(
        &self,
        username: &str,
        password_hash: &str,
        role: Role,
    ) -> Result<CreateUserOutcome, PersistenceError> {
        let result: Result<UserRow, sqlx::Error> = sqlx::query_as(
            "INSERT INTO users (id, username, password_hash, role) VALUES ($1, $2, $3, $4) \
             RETURNING id, username, role, disabled, created_at",
        )
        .bind(Uuid::new_v4())
        .bind(username)
        .bind(password_hash)
        .bind(role.as_str())
        .fetch_one(self.pool())
        .await;
        match result {
            Ok(row) => Ok(CreateUserOutcome::Created(user_from_row(row)?)),
            Err(err) if is_unique_violation(&err) => Ok(CreateUserOutcome::UsernameTaken),
            Err(err) => Err(err.into()),
        }
    }

    /// Konto und Passwort-Hash für die Anmeldung.
    pub async fn find_credentials(
        &self,
        username: &str,
    ) -> Result<Option<(User, String)>, PersistenceError> {
        let row: Option<CredentialRow> = sqlx::query_as(
            "SELECT id, username, role, disabled, created_at, password_hash FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(self.pool())
        .await?;
        row.map(|(id, username, role, disabled, created_at, hash)| {
            Ok((
                user_from_row((id, username, role, disabled, created_at))?,
                hash,
            ))
        })
        .transpose()
    }

    pub async fn set_user_disabled(
        &self,
        username: &str,
        disabled: bool,
    ) -> Result<bool, PersistenceError> {
        Ok(
            sqlx::query("UPDATE users SET disabled = $2, updated_at = now() WHERE username = $1")
                .bind(username)
                .bind(disabled)
                .execute(self.pool())
                .await?
                .rows_affected()
                == 1,
        )
    }

    /// Aktiviert/deaktiviert ein Konto. Beim Deaktivieren werden alle Sitzungen
    /// sofort beendet; das letzte aktive Admin-Konto bleibt geschützt.
    pub async fn set_user_disabled_by_id(
        &self,
        id: Uuid,
        disabled: bool,
    ) -> Result<DisableOutcome, PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some(role) = role else {
            return Ok(DisableOutcome::NotFound);
        };
        if disabled && role == "admin" {
            let other_admins: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM users WHERE role = 'admin' AND NOT disabled AND id <> $1",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            if other_admins == 0 {
                return Ok(DisableOutcome::LastAdmin);
            }
        }
        let row: UserRow = sqlx::query_as(
            "UPDATE users SET disabled = $2, updated_at = now() WHERE id = $1 \
             RETURNING id, username, role, disabled, created_at",
        )
        .bind(id)
        .bind(disabled)
        .fetch_one(&mut *tx)
        .await?;
        if disabled {
            sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(DisableOutcome::Updated(user_from_row(row)?))
    }

    /// Setzt einen neuen Passwort-Hash und beendet alle anderen Sitzungen des Kontos.
    pub async fn change_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
        keep_session_hash: &str,
    ) -> Result<u64, PersistenceError> {
        let mut tx = self.pool().begin().await?;
        sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
            .bind(user_id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?;
        let ended = sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2")
            .bind(user_id)
            .bind(keep_session_hash)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        Ok(ended)
    }

    pub async fn list_users(&self) -> Result<Vec<User>, PersistenceError> {
        let rows: Vec<UserRow> = sqlx::query_as(
            "SELECT id, username, role, disabled, created_at FROM users ORDER BY created_at",
        )
        .fetch_all(self.pool())
        .await?;
        rows.into_iter().map(user_from_row).collect()
    }

    pub async fn create_session(
        &self,
        token_hash: &str,
        user_id: Uuid,
        csrf_token: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), PersistenceError> {
        sqlx::query(
            "INSERT INTO sessions (token_hash, user_id, csrf_token, expires_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(token_hash)
        .bind(user_id)
        .bind(csrf_token)
        .bind(expires_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Gültige Sitzung eines aktiven Kontos; `idle_cutoff` begrenzt die Inaktivität.
    pub async fn find_session(
        &self,
        token_hash: &str,
        idle_cutoff: DateTime<Utc>,
    ) -> Result<Option<SessionRecord>, PersistenceError> {
        let row: Option<SessionRow> =
            sqlx::query_as(
                "SELECT u.id, u.username, u.role, u.disabled, u.created_at, s.csrf_token, s.last_seen_at \
                 FROM sessions s JOIN users u ON u.id = s.user_id \
                 WHERE s.token_hash = $1 AND s.expires_at > now() AND s.last_seen_at > $2 \
                   AND NOT u.disabled",
            )
            .bind(token_hash)
            .bind(idle_cutoff)
            .fetch_optional(self.pool())
            .await?;
        row.map(
            |(id, username, role, disabled, created_at, csrf_token, last_seen_at)| {
                Ok(SessionRecord {
                    user: user_from_row((id, username, role, disabled, created_at))?,
                    csrf_token,
                    last_seen_at,
                })
            },
        )
        .transpose()
    }

    pub async fn touch_session(&self, token_hash: &str) -> Result<(), PersistenceError> {
        sqlx::query("UPDATE sessions SET last_seen_at = now() WHERE token_hash = $1")
            .bind(token_hash)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_session(&self, token_hash: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_expired_sessions(&self) -> Result<u64, PersistenceError> {
        Ok(
            sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
                .execute(self.pool())
                .await?
                .rows_affected(),
        )
    }

    pub async fn audit(
        &self,
        actor: &str,
        action: &str,
        target: Option<&str>,
        details: serde_json::Value,
        correlation_id: Option<&str>,
    ) -> Result<(), PersistenceError> {
        sqlx::query(
            "INSERT INTO audit_log (actor, action, target, details, correlation_id) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(actor)
        .bind(action)
        .bind(target)
        .bind(details)
        .bind(correlation_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    pub async fn audit_actions(&self) -> Result<Vec<(String, String)>, PersistenceError> {
        Ok(
            sqlx::query_as("SELECT actor, action FROM audit_log ORDER BY id")
                .fetch_all(self.pool())
                .await?,
        )
    }
}
