//! Content-Security-Policy ohne `'unsafe-inline'` (AKL-M020-07).
//!
//! Die gebaute SPA enthält genau ein Inline-Skript (SvelteKit-Bootstrap), dessen
//! Inhalt sich mit jedem Build ändert. Statt eines festen Hashes berechnet die
//! API beim Start die SHA-256-Hashes aller Inline-Skripte aus `index.html` und
//! erlaubt nur diese.
//!
//! Inline-Styles: Nur feste `style="…"`-Literale aus den gebauten Dateien (z. B.
//! der unsichtbare Routen-Ansager von SvelteKit) werden per `'unsafe-hashes'`
//! und Hash **genau dieses Werts** erlaubt – nie beliebige Inline-Styles.

use std::{collections::BTreeSet, path::Path};

use base64::Engine as _;
use sha2::{Digest, Sha256};

/// Größte Datei, die nach Style-Literalen durchsucht wird.
const MAX_SCAN_BYTES: u64 = 8 * 1024 * 1024;

/// Policy für die ausgelieferte UI bzw. ohne UI (nur API).
pub fn policy(web_dir: Option<&Path>) -> String {
    let script_hashes = web_dir
        .and_then(
            |dir| match std::fs::read_to_string(dir.join("index.html")) {
                Ok(html) => Some(inline_script_hashes(&html)),
                Err(err) => {
                    tracing::warn!(%err, "index.html nicht lesbar – CSP ohne Skript-Hashes");
                    None
                }
            },
        )
        .unwrap_or_default();
    let style_hashes = web_dir.map(style_attribute_hashes).unwrap_or_default();

    let mut script_src = String::from("'self'");
    for hash in &script_hashes {
        script_src.push_str(&format!(" '{hash}'"));
    }
    let mut style_src = String::from("'self'");
    if !style_hashes.is_empty() {
        style_src.push_str(" 'unsafe-hashes'");
        for hash in &style_hashes {
            style_src.push_str(&format!(" '{hash}'"));
        }
    }
    format!(
        "default-src 'self'; script-src {script_src}; style-src {style_src}; \
         img-src 'self' data:; connect-src 'self'; object-src 'none'; \
         frame-ancestors 'none'; base-uri 'self'; form-action 'self'"
    )
}

fn sha256_source(value: &str) -> String {
    format!(
        "sha256-{}",
        base64::engine::general_purpose::STANDARD.encode(Sha256::digest(value.as_bytes()))
    )
}

/// Hashes aller festen `style="…"`-Literale in `.html`/`.js` unter `dir`.
fn style_attribute_hashes(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(p);
            } else if meta.len() <= MAX_SCAN_BYTES
                && matches!(p.extension().and_then(|e| e.to_str()), Some("js" | "html"))
                && let Ok(text) = std::fs::read_to_string(&p)
            {
                out.extend(style_literals(&text).into_iter().map(sha256_source));
            }
        }
    }
    out
}

/// Feste Werte von `style="…"`; Werte mit Platzhaltern (`${`, `{`) zählen nicht.
fn style_literals(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("style=\"") {
        rest = &rest[start + "style=\"".len()..];
        let Some(end) = rest.find('"') else { break };
        let value = &rest[..end];
        rest = &rest[end + 1..];
        if !value.is_empty() && !value.contains('{') {
            out.push(value);
        }
    }
    out
}

/// `sha256-…`-Quellen für alle `<script>`-Elemente ohne `src`.
fn inline_script_hashes(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        rest = &rest[start + "<script".len()..];
        let Some(tag_end) = rest.find('>') else { break };
        let attrs = &rest[..tag_end];
        rest = &rest[tag_end + 1..];
        let Some(close) = rest.find("</script>") else {
            break;
        };
        let body = &rest[..close];
        rest = &rest[close + "</script>".len()..];
        let external = attrs
            .split(|c: char| c.is_whitespace())
            .any(|a| a.starts_with("src="));
        if !external {
            out.push(sha256_source(body));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_only_inline_scripts_exactly() {
        let html = "<head><script src=\"/theme-init.js\"></script>\
                    <script type=\"module\">\n\tboot();\n</script></head>";
        let hashes = inline_script_hashes(html);
        let expected = base64::engine::general_purpose::STANDARD
            .encode(Sha256::digest("\n\tboot();\n".as_bytes()));
        assert_eq!(hashes, vec![format!("sha256-{expected}")]);
    }

    #[test]
    fn policy_never_allows_unsafe_inline() {
        let dir = std::env::temp_dir().join(format!("akl-csp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<script>a()</script>").unwrap();
        let with_ui = policy(Some(&dir));
        assert!(with_ui.contains("script-src 'self' 'sha256-"), "{with_ui}");
        assert!(with_ui.contains("style-src 'self';"));
        std::fs::remove_dir_all(&dir).unwrap();

        for p in [
            with_ui,
            policy(None),
            policy(Some(Path::new("/nonexistent"))),
        ] {
            assert!(!p.contains("unsafe-inline"), "{p}");
            assert!(p.contains("frame-ancestors 'none'"));
            assert!(p.contains("object-src 'none'"));
        }
    }

    #[test]
    fn unterminated_script_is_ignored() {
        assert!(inline_script_hashes("<script>never closed").is_empty());
        assert!(inline_script_hashes("<script").is_empty());
    }

    #[test]
    fn style_literals_only_fixed_values() {
        let js = r#"b(`<div style="width: 1px; height: 1px"></div>`); x(`<p style="color: ${c}">`);
                    y(`<i style="">`); z(`<b style="a: {b}">`)"#;
        assert_eq!(style_literals(js), vec!["width: 1px; height: 1px"]);
    }

    #[test]
    fn style_hashes_use_unsafe_hashes_never_unsafe_inline() {
        // Wert und Hash des SvelteKit-Routen-Ansagers (im Browser gemessen).
        let announcer = "position: absolute; left: 0; top: 0; clip: rect(0 0 0 0); \
                         clip-path: inset(50%); overflow: hidden; white-space: nowrap; \
                         width: 1px; height: 1px";
        assert_eq!(
            sha256_source(announcer),
            "sha256-S8qMpvofolR8Mpjy4kQvEm7m1q8clzU4dfDH0AmvZjo="
        );

        let dir = std::env::temp_dir().join(format!("akl-csp-style-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("_app/immutable")).unwrap();
        std::fs::write(dir.join("index.html"), "<p>ok</p>").unwrap();
        std::fs::write(
            dir.join("_app/immutable/a.js"),
            format!("t(`<div style=\"{announcer}\"></div>`)"),
        )
        .unwrap();
        let p = policy(Some(&dir));
        assert!(
            p.contains(
                "style-src 'self' 'unsafe-hashes' 'sha256-S8qMpvofolR8Mpjy4kQvEm7m1q8clzU4dfDH0AmvZjo=';"
            ),
            "{p}"
        );
        assert!(!p.contains("unsafe-inline"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
