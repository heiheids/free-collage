use std::collections::HashMap;
use std::path::PathBuf;

const EMBEDDED_ZH: &str = include_str!("../languages/zh-CN.lang");
const EMBEDDED_EN: &str = include_str!("../languages/en-US.lang");

#[derive(Clone, Debug)]
pub struct Lang {
    pub code: String,
    pub name: String,
    pub map: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct LangInfo {
    pub code: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct I18n {
    current: Lang,
    fallback: Lang,
    available: Vec<LangInfo>,
}

impl I18n {
    pub fn load(preferred: &str, system_code: &str) -> Self {
        let mut langs: Vec<Lang> = Vec::new();
        for dir in language_dirs() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut files: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("lang"))
                .collect();
            files.sort();
            for path in files {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                if let Some(lang) = parse_lang(&text) {
                    if !langs.iter().any(|l| l.code == lang.code) {
                        langs.push(lang);
                    }
                }
            }
        }
        for text in [EMBEDDED_ZH, EMBEDDED_EN] {
            if let Some(lang) = parse_lang(text) {
                if !langs.iter().any(|l| l.code == lang.code) {
                    langs.push(lang);
                }
            }
        }

        let want = if preferred == "auto" {
            system_code
        } else {
            preferred
        };
        let mut current = langs
            .iter()
            .find(|l| l.code.eq_ignore_ascii_case(want))
            .cloned()
            .or_else(|| {
                if want.to_ascii_lowercase().starts_with("zh") {
                    langs.iter().find(|l| l.code.eq_ignore_ascii_case("zh-CN")).cloned()
                } else {
                    None
                }
            })
            .or_else(|| langs.iter().find(|l| l.code.eq_ignore_ascii_case("en-US")).cloned())
            .or_else(|| langs.first().cloned())
            .unwrap_or_else(|| parse_lang(EMBEDDED_EN).expect("embedded language"));

        let fallback = langs
            .iter()
            .find(|l| l.code.eq_ignore_ascii_case("en-US"))
            .cloned()
            .unwrap_or_else(|| current.clone());

        let mut available: Vec<LangInfo> = langs
            .iter()
            .map(|l| LangInfo {
                code: l.code.clone(),
                name: l.name.clone(),
            })
            .collect();
        available.sort_by(|a, b| {
            let rank = |c: &str| {
                if c.eq_ignore_ascii_case("zh-CN") {
                    0
                } else if c.eq_ignore_ascii_case("en-US") {
                    1
                } else {
                    2
                }
            };
            rank(&a.code).cmp(&rank(&b.code)).then(a.name.cmp(&b.name))
        });
        available.dedup_by(|a, b| a.code == b.code);

        if current.name.is_empty() {
            current.name = current.code.clone();
        }
        Self {
            current,
            fallback,
            available,
        }
    }

    pub fn current_code(&self) -> &str {
        &self.current.code
    }

    pub fn current_name(&self) -> &str {
        &self.current.name
    }

    pub fn available(&self) -> &[LangInfo] {
        &self.available
    }

    pub fn set(&mut self, code: &str) -> bool {
        if self.current.code.eq_ignore_ascii_case(code) {
            return true;
        }
        if let Some(info) = self
            .available
            .iter()
            .find(|l| l.code.eq_ignore_ascii_case(code))
            .cloned()
        {
            if let Some(lang) = load_lang_by_code(&info.code) {
                self.current = lang;
                return true;
            }
        }
        false
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.current
            .map
            .get(key)
            .or_else(|| self.fallback.map.get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }

    pub fn has(&self, key: &str) -> bool {
        self.current.map.contains_key(key) || self.fallback.map.contains_key(key)
    }

    pub fn fmt(&self, key: &str, args: &[&str]) -> String {
        let mut s = self.t(key).to_string();
        for (i, value) in args.iter().enumerate() {
            s = s.replace(&format!("{{{i}}}"), value);
        }
        s
    }
}

fn load_lang_by_code(code: &str) -> Option<Lang> {
    for dir in language_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("lang") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(lang) = parse_lang(&text) {
                if lang.code.eq_ignore_ascii_case(code) {
                    return Some(lang);
                }
            }
        }
    }
    for text in [EMBEDDED_ZH, EMBEDDED_EN] {
        if let Some(lang) = parse_lang(text) {
            if lang.code.eq_ignore_ascii_case(code) {
                return Some(lang);
            }
        }
    }
    None
}

fn language_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("languages"));
            dirs.push(dir.join("../../languages"));
        }
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("languages"));
    let mut seen = Vec::new();
    dirs.retain(|d| {
        let key = d.to_string_lossy().to_lowercase();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    dirs
}

pub fn parse_lang(text: &str) -> Option<Lang> {
    let mut map = HashMap::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            if !k.is_empty() {
                map.insert(k.to_string(), v.trim().to_string());
            }
        }
    }
    let code = map.get("code")?.trim().to_string();
    let name = map.get("name").cloned().unwrap_or_else(|| code.clone());
    Some(Lang { code, name, map })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_parses() {
        let l = parse_lang("code=xx\nname=Test\nhello=World\n").unwrap();
        assert_eq!(l.code, "xx");
        assert_eq!(l.map.get("hello").unwrap(), "World");
    }

    #[test]
    fn key_sets_match() {
        let zh = parse_lang(EMBEDDED_ZH).unwrap();
        let en = parse_lang(EMBEDDED_EN).unwrap();
        let mut zh_keys: Vec<_> = zh.map.keys().collect();
        let mut en_keys: Vec<_> = en.map.keys().collect();
        zh_keys.sort();
        en_keys.sort();
        assert_eq!(zh_keys, en_keys);
    }

    #[test]
    fn intros_are_short_enough() {
        let zh = parse_lang(EMBEDDED_ZH).unwrap();
        let en = parse_lang(EMBEDDED_EN).unwrap();
        let zh_intro = zh.map.get("about.intro_text").unwrap();
        let en_intro = en.map.get("about.intro_text").unwrap();
        assert!(zh_intro.chars().count() <= 100, "zh intro too long");
        assert!(en_intro.split_whitespace().count() <= 100, "en intro too long");
    }
}
