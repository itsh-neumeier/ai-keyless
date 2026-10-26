//! Rollen und Eingaberegeln für Konten (AKL-M020).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Viewer,
    Operator,
    Admin,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
            Self::Admin => "admin",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Viewer => 0,
            Self::Operator => 1,
            Self::Admin => 2,
        }
    }

    /// Darf diese Rolle eine Aktion ausführen, die `required` verlangt?
    pub fn allows(self, required: Role) -> bool {
        self.rank() >= required.rank()
    }
}

impl std::str::FromStr for Role {
    type Err = crate::device::UnknownValue;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "viewer" => Ok(Self::Viewer),
            "operator" => Ok(Self::Operator),
            "admin" => Ok(Self::Admin),
            other => Err(crate::device::UnknownValue(other.to_owned())),
        }
    }
}

pub const PASSWORD_MIN_CHARS: usize = 12;
pub const PASSWORD_MAX_CHARS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CredentialError {
    #[error("Benutzername: 3–64 Zeichen aus a–z, 0–9, . _ -")]
    InvalidUsername,
    #[error("Passwort: mindestens 12 und höchstens 256 Zeichen")]
    InvalidPassword,
}

impl CredentialError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidUsername => "invalid_username",
            Self::InvalidPassword => "invalid_password",
        }
    }
}

/// Normalisiert (Kleinbuchstaben, ohne Rand-Leerraum) und prüft einen Benutzernamen.
/// Muss mit dem CHECK-Constraint der Tabelle `users` übereinstimmen.
pub fn normalize_username(input: &str) -> Result<String, CredentialError> {
    let name = input.trim().to_lowercase();
    let valid_len = (3..=64).contains(&name.len());
    let valid_chars = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    if valid_len && valid_chars {
        Ok(name)
    } else {
        Err(CredentialError::InvalidUsername)
    }
}

pub fn validate_password(password: &str) -> Result<(), CredentialError> {
    let chars = password.chars().count();
    if (PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&chars) {
        Ok(())
    } else {
        Err(CredentialError::InvalidPassword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_hierarchy() {
        assert!(Role::Admin.allows(Role::Operator));
        assert!(Role::Operator.allows(Role::Viewer));
        assert!(!Role::Viewer.allows(Role::Operator));
        assert!(!Role::Operator.allows(Role::Admin));
        assert_eq!("operator".parse::<Role>().unwrap(), Role::Operator);
        assert!("root".parse::<Role>().is_err());
    }

    #[test]
    fn usernames() {
        assert_eq!(normalize_username("  Admin ").unwrap(), "admin");
        assert_eq!(normalize_username("ops.team-1_x").unwrap(), "ops.team-1_x");
        for bad in ["ab", "with space", "ümlaut", "a/b", "", &"x".repeat(65)] {
            assert!(normalize_username(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn passwords() {
        assert!(validate_password("kurz").is_err());
        assert!(validate_password("genau-12-zei").is_ok());
        // Zählt Zeichen, nicht Bytes.
        assert!(validate_password("äöüäöüäöüäö").is_err());
        assert!(validate_password(&"x".repeat(257)).is_err());
    }
}
