//! Minimal ini editor: patches single keys while preserving everything else
//! (comments, section order, BOM, CRLF). Used for ppsspp.ini, which PPSSPP
//! owns — we may only touch the keys we mean to.

use std::io;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Ini {
    text: String,
}

impl Ini {
    pub fn load(path: &Path) -> io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Self { text }),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e),
        }
    }

    /// `(section, body lines)` spans: byte range of lines belonging to each
    /// `[section]` (excluding the header line itself).
    fn sections(&self) -> Vec<(String, usize, usize)> {
        let mut out = Vec::new();
        let mut cur: Option<(String, usize)> = None;
        for (off, line) in self.line_spans() {
            let t = line.trim_start_matches('\u{feff}').trim();
            if t.starts_with('[') && t.contains(']') {
                if let Some((name, start)) = cur.take() {
                    out.push((name, start, off));
                }
                let end = t.find(']').unwrap();
                cur = Some((t[1..end].trim().to_string(), off + line.len()));
            }
        }
        if let Some((name, start)) = cur {
            out.push((name, start, self.text.len()));
        }
        out
    }
    fn line_spans(&self) -> Vec<(usize, &str)> {
        let mut v = Vec::new();
        let mut off = 0;
        for line in self.text.split_inclusive('\n') {
            v.push((off, line));
            off += line.len();
        }
        v
    }


    fn key_of(line: &str) -> Option<&str> {
        let l = line.trim_start_matches('\u{feff}').trim_start();
        if l.starts_with('[') || l.starts_with('#') || l.starts_with(';') {
            return None;
        }
        let eq = l.find('=')?;
        Some(l[..eq].trim())
    }

    pub fn get(&self, section: &str, key: &str) -> Option<String> {
        let mut in_sec = false;
        for (_, line) in self.line_spans() {
            let t = line.trim_start_matches('\u{feff}').trim();
            if t.starts_with('[') && t.contains(']') {
                let end = t.find(']').unwrap();
                in_sec = t[1..end].trim() == section;
                continue;
            }
            if in_sec && Self::key_of(line) == Some(key) {
                let eq = line.find('=').unwrap();
                return Some(line[eq + 1..].trim().to_string());
            }
        }
        None
    }

    /// Replaces the `key = value` line inside `[section]`; appends the key at
    /// the end of the section, or the section at the end of the file.
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let newline = if self.text.contains("\r\n") { "\r\n" } else { "\n" };
        let secs = self.sections();

        if let Some((_, start, end)) = secs.iter().find(|(n, _, _)| n == section) {
            // find key line inside the section body
            for (off, line) in self.line_spans() {
                if off < *start || off >= *end {
                    continue;
                }
                if Self::key_of(line) == Some(key) {
                    self.text
                        .replace_range(off..off + line.len(), &format!("{key} = {value}{newline}"));
                    return;
                }
            }
            // key missing: insert before the section's last line (keeps order tidy)
            self.text
                .insert_str(*end, &format!("{key} = {value}{newline}"));
            return;
        }

        // section missing: append header + key (and a BOM on a fresh file)
        if self.text.is_empty() {
            self.text.push('\u{feff}');
        } else if !self.text.ends_with('\n') {
            self.text.push_str(newline);
        }
        self.text
            .push_str(&format!("[{section}]{newline}{key} = {value}{newline}"));
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let tmp = path.with_extension("ini.tmp");
        std::fs::write(&tmp, &self.text)?;
        std::fs::rename(&tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_bom_comments_and_crlf() {
        let src = "\u{feff}[General]\r\n; a comment\r\nFoo = 1\r\n\r\n[Graphics]\r\nFrameSkip = 0\r\nFrameSkipType = 1\r\n";
        let mut ini = Ini { text: src.into() };
        ini.set("Graphics", "FrameSkip", "2");
        // only the FrameSkip line changed; FrameSkipType untouched
        assert!(ini.text.contains("FrameSkip = 2\r\n"));
        assert!(ini.text.contains("FrameSkipType = 1\r\n"));
        assert!(ini.text.starts_with('\u{feff}'));
        assert!(ini.text.contains("; a comment"));
        assert_eq!(ini.get("Graphics", "FrameSkip").as_deref(), Some("2"));
        assert_eq!(ini.get("Graphics", "FrameSkipType").as_deref(), Some("1"));
    }

    #[test]
    fn inserts_missing_key_before_next_section() {
        let mut ini = Ini { text: "[A]\nx = 1\n[B]\ny = 2\n".into() };
        ini.set("A", "New", "v");
        let a = ini.text.find("New = v").unwrap();
        let b = ini.text.find("[B]").unwrap();
        assert!(a < b);
        assert_eq!(ini.get("A", "New").as_deref(), Some("v"));
        assert_eq!(ini.get("B", "y").as_deref(), Some("2"));
    }

    #[test]
    fn creates_missing_section_and_file() {
        let mut ini = Ini::default();
        ini.set("Sound", "GameVolume", "50");
        assert_eq!(ini.get("Sound", "GameVolume").as_deref(), Some("50"));
        assert!(ini.text.contains("[Sound]"));
    }

    #[test]
    fn reads_translated_enum_value() {
        let ini = Ini { text: "[Graphics]\nGraphicsBackend = 0 (OPENGL)\n".into() };
        assert_eq!(ini.get("Graphics", "GraphicsBackend").as_deref(), Some("0 (OPENGL)"));
    }
}
