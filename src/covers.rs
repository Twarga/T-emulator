//! Box-art lookup + download for the library.
//! Covers live as <slug>.png in the data dir (downloaded at runtime) or in the
//! repo's assets/covers (bundled/dev). Art comes from libretro-thumbnails.

use std::path::PathBuf;

/// "Crisis Core - Final Fantasy VII" -> "crisis-core-final-fantasy-vii"
pub fn slug(title: &str) -> String {
    let mut s = String::with_capacity(title.len());
    let mut dash = false;
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            if dash && !s.is_empty() {
                s.push('-');
            }
            dash = false;
            s.push(c);
        } else {
            dash = true;
        }
    }
    s
}

pub fn cache_dir() -> PathBuf {
    crate::config::Config::emulators_dir()
        .parent()
        .map(|p| p.join("covers"))
        .unwrap_or_else(|| ".covers".into())
}

/// First existing cover file: downloaded cache, then dev assets dir.
pub fn path_for(title: &str) -> Option<PathBuf> {
    let f = format!("{}.png", slug(title));
    let cache = cache_dir().join(&f);
    if cache.exists() {
        return Some(cache);
    }
    for base in [PathBuf::from("assets/covers"), exe_rel("assets/covers")] {
        let p = base.join(&f);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn exe_rel(rel: &str) -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(rel)))
        .unwrap_or_default()
}

/// libretro-thumbnails filename candidates, best region first.
fn candidates(title: &str) -> Vec<String> {
    const REGIONS: &[&str] = &[
        "",
        " (USA)",
        " (USA) (En,Fr,De,Es,It)",
        " (USA) (En,Fr,Es)",
        " (USA, Korea)",
        " (Europe)",
        " (Europe) (En,Fr,De,Es,It)",
        " (Japan)",
        " (World)",
    ];
    let t = title.replace(':', "_").replace('&', "_");
    REGIONS.iter().map(|r| format!("{t}{r}")).collect()
}

const BASE: &str =
    "https://raw.githubusercontent.com/libretro-thumbnails/Sony_-_PlayStation_Portable/master/Named_Boxarts";

/// Download covers for titles missing one. Blocking; call off the UI thread.
/// Returns titles that got a cover.
pub fn fetch_missing(titles: &[String]) -> Vec<String> {
    let dir = cache_dir();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(20)))
        .build()
        .into();
    let mut got = Vec::new();
    for title in titles {
        if path_for(title).is_some() {
            continue;
        }
        for cand in candidates(title) {
            let url = format!("{BASE}/{}.png", urlenc(&cand));
            let Ok(mut resp) = agent.get(&url).call() else { continue };
            if resp.status().as_u16() != 200 {
                continue;
            }
            let Ok(bytes) = resp.body_mut().read_to_vec() else { continue };
            if bytes.len() < 8_000 {
                continue; // 404 page or junk
            }
            let _ = std::fs::create_dir_all(&dir);
            let out = dir.join(format!("{}.png", slug(title)));
            if std::fs::write(&out, &bytes).is_ok() {
                got.push(title.clone());
            }
            break;
        }
    }
    got
}

fn urlenc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_normalizes() {
        assert_eq!(slug("Crisis Core - Final Fantasy VII"), "crisis-core-final-fantasy-vii");
        assert_eq!(slug("God of War  Ghost of Sparta"), "god-of-war-ghost-of-sparta");
        assert_eq!(slug("Patapon 3"), "patapon-3");
    }
}
