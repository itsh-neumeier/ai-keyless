//! Persistente Geräteidentität je Profil (AKL-M040-01, Bedrohung T5).
//!
//! Jedes Profil (`ai_key`, `ai_port`; ADR-003) bekommt ein eigenes
//! Schlüsselpaar mit selbstsigniertem Zertifikat. Beides liegt **nur** im
//! Identitätsverzeichnis (`AKL_DEVICE_DIR`), nie in der Datenbank:
//!
//! ```text
//! <root>/<profile_id>/identity.json  öffentlich: Profilart, Fingerprint, Zeitpunkt
//! <root>/<profile_id>/cert.pem       öffentlich
//! <root>/<profile_id>/key.pem        privat, Modus 0600
//! ```
//!
//! Angelegt wird in einem temporären Verzeichnis, das erst vollständig
//! geschrieben umbenannt wird – ein Absturz hinterlässt nie eine halbe
//! Identität. Beim Laden wird geprüft, dass Profilart, Zertifikat und
//! Schlüssel zusammenpassen; eine beschädigte Identität wird nie still
//! ersetzt (sonst müsste Protect das Gerät neu adoptieren).
//!
//! Welche Schlüsselart und welche Zertifikatsfelder Protect erwartet, ist
//! unbekannt (`needs_evidence`, Q-006/Q-007). Bis dahin: ECDSA P-256,
//! neutraler Name ohne Herstellerbezug.

use std::{
    fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use akl_core::device::DeviceProfileKind;
use akl_protect_compat::tls::Fingerprint;
use chrono::{DateTime, Utc};
use rcgen::{CertificateParams, DnType, KeyPair, PublicKeyData};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const META: &str = "identity.json";
const CERT: &str = "cert.pem";
const KEY: &str = "key.pem";
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("Identitätsverzeichnis: {0}")]
    Io(#[from] io::Error),
    #[error("Identität {profile_id} ist beschädigt: {reason}")]
    Corrupt { profile_id: Uuid, reason: String },
    #[error("Identität {profile_id} gehört zu Profil {found}, erwartet {expected}")]
    KindMismatch {
        profile_id: Uuid,
        expected: &'static str,
        found: String,
    },
    #[error("Schlüssel/Zertifikat erzeugen: {0}")]
    Generate(#[from] rcgen::Error),
}

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    version: u32,
    profile_id: Uuid,
    kind: String,
    fingerprint: String,
    created_at: DateTime<Utc>,
}

/// Privater Schlüssel (PKCS#8, DER). `Debug` gibt ihn nie aus.
struct SecretKey(Vec<u8>);

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[redacted]")
    }
}

/// Geladene Identität eines Profils.
#[derive(Debug)]
pub struct DeviceIdentity {
    pub profile_id: Uuid,
    pub kind: DeviceProfileKind,
    pub fingerprint: Fingerprint,
    pub created_at: DateTime<Utc>,
    cert_der: Vec<u8>,
    key: SecretKey,
}

impl DeviceIdentity {
    pub fn cert_der(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.cert_der.clone())
    }

    /// Privater Schlüssel für rustls. Nur an TLS übergeben, nie ausgeben.
    pub fn private_key_der(&self) -> PrivateKeyDer<'static> {
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(self.key.0.clone()))
    }
}

/// Ergebnis von [`IdentityStore::load_or_create`].
#[derive(Debug)]
pub struct Loaded {
    pub identity: DeviceIdentity,
    /// `true`, wenn die Identität gerade neu erzeugt wurde.
    pub created: bool,
}

/// Dateibasierter Speicher für Geräteidentitäten.
#[derive(Debug, Clone)]
pub struct IdentityStore {
    root: PathBuf,
}

impl IdentityStore {
    /// Öffnet (und legt bei Bedarf an) das Identitätsverzeichnis, Modus 0700.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, IdentityError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        restrict_dir(&root)?;
        Ok(Self { root })
    }

    fn dir(&self, profile_id: Uuid) -> PathBuf {
        self.root.join(profile_id.to_string())
    }

    /// Lädt die Identität des Profils; `None`, wenn noch keine existiert.
    pub fn load(
        &self,
        profile_id: Uuid,
        kind: DeviceProfileKind,
    ) -> Result<Option<DeviceIdentity>, IdentityError> {
        let dir = self.dir(profile_id);
        if !dir.exists() {
            return Ok(None);
        }
        read_identity(&dir, profile_id, kind).map(Some)
    }

    /// Lädt die Identität oder erzeugt sie beim ersten Aufruf.
    pub fn load_or_create(
        &self,
        profile_id: Uuid,
        kind: DeviceProfileKind,
    ) -> Result<Loaded, IdentityError> {
        if let Some(identity) = self.load(profile_id, kind)? {
            return Ok(Loaded {
                identity,
                created: false,
            });
        }
        let tmp = self.root.join(format!(".tmp-{}", Uuid::new_v4()));
        let result = write_new(&tmp, profile_id, kind).and_then(|()| {
            fs::rename(&tmp, self.dir(profile_id))?;
            Ok(())
        });
        if let Err(err) = result {
            let _ = fs::remove_dir_all(&tmp);
            return Err(err);
        }
        sync_dir(&self.root)?;
        let identity = read_identity(&self.dir(profile_id), profile_id, kind)?;
        Ok(Loaded {
            identity,
            created: true,
        })
    }
}

fn write_new(dir: &Path, profile_id: Uuid, kind: DeviceProfileKind) -> Result<(), IdentityError> {
    fs::create_dir(dir)?;
    restrict_dir(dir)?;

    let key = KeyPair::generate()?;
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    let short = &profile_id.simple().to_string()[..8];
    params.distinguished_name.push(
        DnType::CommonName,
        format!("ai-keyless-{}-{short}", kind.as_str()),
    );
    let cert = params.self_signed(&key)?;

    let meta = Meta {
        version: FORMAT_VERSION,
        profile_id,
        kind: kind.as_str().to_owned(),
        fingerprint: Fingerprint::of(cert.der()).to_string(),
        created_at: Utc::now(),
    };
    // Schlüssel zuerst und nur mit 0600 anlegen; nie mit weiteren Rechten.
    write_file(&dir.join(KEY), key.serialize_pem().as_bytes(), true)?;
    write_file(&dir.join(CERT), cert.pem().as_bytes(), false)?;
    let json = serde_json::to_vec_pretty(&meta).map_err(io::Error::other)?;
    write_file(&dir.join(META), &json, false)?;
    sync_dir(dir)?;
    Ok(())
}

fn read_identity(
    dir: &Path,
    profile_id: Uuid,
    kind: DeviceProfileKind,
) -> Result<DeviceIdentity, IdentityError> {
    let corrupt = |reason: String| IdentityError::Corrupt { profile_id, reason };

    let meta: Meta = serde_json::from_slice(&fs::read(dir.join(META))?)
        .map_err(|e| corrupt(format!("{META}: {e}")))?;
    if meta.version != FORMAT_VERSION {
        return Err(corrupt(format!(
            "unbekannte Formatversion {}",
            meta.version
        )));
    }
    if meta.profile_id != profile_id {
        return Err(corrupt(format!("{META} nennt Profil {}", meta.profile_id)));
    }
    if meta.kind != kind.as_str() {
        return Err(IdentityError::KindMismatch {
            profile_id,
            expected: kind.as_str(),
            found: meta.kind,
        });
    }

    let cert_pem = fs::read_to_string(dir.join(CERT))?;
    let cert_der = pem_body(&cert_pem, "CERTIFICATE")
        .ok_or_else(|| corrupt(format!("{CERT}: kein Zertifikat")))?;
    let fingerprint = Fingerprint::of(&cert_der);
    if fingerprint.to_string() != meta.fingerprint {
        return Err(corrupt(format!("{CERT} passt nicht zum Fingerprint")));
    }

    let key_pem = fs::read_to_string(dir.join(KEY))?;
    let key = KeyPair::from_pem(&key_pem).map_err(|e| corrupt(format!("{KEY}: {e}")))?;
    // Das SubjectPublicKeyInfo des Schlüssels steht wörtlich im Zertifikat.
    let spki = key.subject_public_key_info();
    if !cert_der.windows(spki.len()).any(|w| w == spki.as_slice()) {
        return Err(corrupt(format!("{KEY} passt nicht zu {CERT}")));
    }

    Ok(DeviceIdentity {
        profile_id,
        kind,
        fingerprint,
        created_at: meta.created_at,
        cert_der,
        key: SecretKey(key.serialize_der()),
    })
}

/// Dekodiert den ersten PEM-Block mit dem angegebenen Etikett.
fn pem_body(pem: &str, label: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = pem.find(&begin)? + begin.len();
    let stop = start + pem[start..].find(&end)?;
    let b64: String = pem[start..stop]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64::engine::general_purpose::STANDARD.decode(b64).ok()
}

fn write_file(path: &Path, data: &[u8], private: bool) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if private { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut file = options.open(path)?;
    file.write_all(data)?;
    file.sync_all()
}

fn restrict_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeviceProfileKind::*;

    fn store() -> (IdentityStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("akl-identity-{}", Uuid::new_v4()));
        (IdentityStore::open(&root).unwrap(), root)
    }

    #[test]
    fn identity_survives_reopen() {
        let (store, root) = store();
        let id = Uuid::new_v4();
        let first = store.load_or_create(id, AiKey).unwrap();
        assert!(first.created);
        drop(store);

        // "Neustart": frischer Speicher auf demselben Verzeichnis.
        let again = IdentityStore::open(&root)
            .unwrap()
            .load_or_create(id, AiKey)
            .unwrap();
        assert!(!again.created);
        assert_eq!(again.identity.fingerprint, first.identity.fingerprint);
        assert_eq!(again.identity.cert_der(), first.identity.cert_der());
        assert_eq!(
            again.identity.private_key_der().secret_der(),
            first.identity.private_key_der().secret_der()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn profiles_get_separate_identities() {
        let (store, root) = store();
        let key = store.load_or_create(Uuid::new_v4(), AiKey).unwrap();
        let port = store.load_or_create(Uuid::new_v4(), AiPort).unwrap();
        assert_ne!(key.identity.fingerprint, port.identity.fingerprint);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wrong_kind_is_rejected() {
        let (store, root) = store();
        let id = Uuid::new_v4();
        store.load_or_create(id, AiKey).unwrap();
        assert!(matches!(
            store.load(id, AiPort),
            Err(IdentityError::KindMismatch { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_identity_is_none() {
        let (store, root) = store();
        assert!(store.load(Uuid::new_v4(), AiKey).unwrap().is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn swapped_key_is_detected_not_replaced() {
        let (store, root) = store();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        store.load_or_create(a, AiKey).unwrap();
        store.load_or_create(b, AiPort).unwrap();
        let foreign = fs::read(root.join(b.to_string()).join(KEY)).unwrap();
        fs::remove_file(root.join(a.to_string()).join(KEY)).unwrap();
        fs::write(root.join(a.to_string()).join(KEY), foreign).unwrap();

        assert!(matches!(
            store.load_or_create(a, AiKey),
            Err(IdentityError::Corrupt { .. })
        ));
        // Nicht still ersetzt: der fremde Schlüssel liegt unverändert da.
        assert_eq!(
            fs::read(root.join(a.to_string()).join(KEY)).unwrap(),
            fs::read(root.join(b.to_string()).join(KEY)).unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tampered_certificate_is_detected() {
        let (store, root) = store();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        store.load_or_create(a, AiKey).unwrap();
        store.load_or_create(b, AiKey).unwrap();
        let cert_b = fs::read(root.join(b.to_string()).join(CERT)).unwrap();
        fs::write(root.join(a.to_string()).join(CERT), cert_b).unwrap();
        assert!(matches!(
            store.load(a, AiKey),
            Err(IdentityError::Corrupt { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn debug_never_shows_private_key() {
        let (store, root) = store();
        let loaded = store.load_or_create(Uuid::new_v4(), AiKey).unwrap();
        let debug = format!("{loaded:?}");
        assert!(debug.contains("[redacted]"), "{debug}");
        assert!(!debug.contains("PRIVATE KEY"));
        let key_b64 =
            fs::read_to_string(root.join(loaded.identity.profile_id.to_string()).join(KEY))
                .unwrap();
        let body: String = key_b64
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect();
        assert!(!debug.contains(&body[..32]));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn key_file_and_dirs_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let (store, root) = store();
        let id = Uuid::new_v4();
        store.load_or_create(id, AiKey).unwrap();
        let mode = |p: PathBuf| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(root.join(id.to_string()).join(KEY)), 0o600);
        assert_eq!(mode(root.join(id.to_string())), 0o700);
        assert_eq!(mode(root.clone()), 0o700);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn no_temp_dirs_left_behind() {
        let (store, root) = store();
        store.load_or_create(Uuid::new_v4(), AiKey).unwrap();
        let leftovers: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
            .collect();
        assert!(leftovers.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
