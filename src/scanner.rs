use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Game {
    pub title: String,
    pub path: PathBuf,
    pub size: String,
}

pub fn scan(dir: &Path) -> Vec<Game> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Game> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let ext = p.extension()?.to_str()?.to_lowercase();
            if !matches!(ext.as_str(), "iso" | "cso" | "pbp") {
                return None;
            }
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some(Game { title: clean_title(p.file_stem()?.to_str()?), size: human(meta.len()), path: p })
        })
        .collect();
    out.sort_by_key(|g| g.title.to_lowercase());
    out
}

/// "God_of_War-Ghost_of_Sparta (USA)" -> "God of War Ghost of Sparta"
fn clean_title(stem: &str) -> String {
    let mut s = stem.replace(['_', '.'], " ");
    while let (Some(a), Some(b)) = (s.rfind('('), s.rfind(')')) {
        if a < b && b == s.trim_end().len() - 1 {
            s.truncate(a);
        } else {
            break;
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn human(b: u64) -> String {
    const U: [&str; 4] = ["B", "KB", "MB", "GB"];
    let (mut v, mut i) = (b as f64, 0);
    while v >= 1024.0 && i < 3 {
        v /= 1024.0;
        i += 1;
    }
    if i >= 3 { format!("{:.1} {}", v, U[i]) } else { format!("{:.0} {}", v, U[i]) }
}

#[cfg(test)]
mod tests {
    use super::clean_title;
    #[test]
    fn strips_region_and_separators() {
        assert_eq!(clean_title("God_of_War_Ghost_of_Sparta (USA)"), "God of War Ghost of Sparta");
        assert_eq!(clean_title("Patapon 3"), "Patapon 3");
    }
}
