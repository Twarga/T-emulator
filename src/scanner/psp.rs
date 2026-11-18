use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Game { pub title: String, pub path: PathBuf, pub size: String }

pub fn scan(dir: &Path) -> Vec<Game> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        let ex = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
        if matches!(ex.as_str(), "iso" | "cso" | "pbp") {
            let title = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?")
                .replace(['_', '-'], " ").trim().to_string();
            let size = e.metadata().map(|m| human(m.len())).unwrap_or_else(|| "?".into());
            out.push(Game { title, path: p, size });
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    out
}

fn human(b: u64) -> String {
    const U: &[&str] = &["B", "KB", "MB", "GB"];
    let mut v = b as f64; let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 { v /= 1024.0; i += 1; }
    format!("{:.1} {}", v, U[i])
}
