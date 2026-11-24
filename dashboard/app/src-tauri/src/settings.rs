use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub roots: Vec<String>,
    pub language: String, // "fr" | "en"
}

impl Default for Settings {
    fn default() -> Self {
        Settings { roots: vec![], language: "fr".into() }
    }
}

pub fn load(config_dir: &Path) -> Settings {
    std::fs::read_to_string(config_dir.join("settings.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(config_dir: &Path, s: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| format!("settings: {e}"))?;
    let json = serde_json::to_string_pretty(s).map_err(|e| format!("settings: {e}"))?;
    std::fs::write(config_dir.join("settings.json"), json).map_err(|e| format!("settings: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let s = load(&dir.path().join("nope"));
        assert_eq!(s.language, "fr");
        assert!(s.roots.is_empty());
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("cfg");
        let s = Settings { roots: vec!["/work".into()], language: "en".into() };
        save(&cfg, &s).unwrap();
        assert_eq!(load(&cfg), s);
        std::fs::write(cfg.join("settings.json"), "{ not json").unwrap();
        assert_eq!(load(&cfg), Settings::default());
    }

    #[test]
    fn old_settings_with_jira_fields_still_load() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), r#"{"roots":["/w"],"jiraUrl":"https://x","jiraEmail":"a@b.c","language":"en"}"#).unwrap();
        let s = load(dir.path());
        assert_eq!((s.roots, s.language.as_str()), (vec!["/w".to_string()], "en"));
    }
}
