use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "gh-token-switch";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub aliases: Vec<String>,
    pub fingerprints: HashMap<String, String>,
    pub notifications: NotificationConfig,
    pub last_used_alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationConfig {
    pub enabled: bool,
    pub only_when_no_tty: bool,
    pub only_on_implicit_cycle: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            only_when_no_tty: true,
            only_on_implicit_cycle: true,
        }
    }
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(Config::default());
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    let cfg: Config = toml::from_str(&raw)
        .with_context(|| format!("failed to parse config: {}", path.display()))?;
    Ok(cfg)
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let dir = config_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir: {}", dir.display()))?;

    let path = config_path()?;
    let body = toml::to_string_pretty(cfg).context("failed to serialize config")?;
    fs::write(&path, body).with_context(|| format!("failed to write config: {}", path.display()))
}

pub fn ensure_alias(cfg: &mut Config, alias: &str) {
    if !cfg.aliases.iter().any(|a| a == alias) {
        cfg.aliases.push(alias.to_string());
    }
}

fn config_dir() -> Result<PathBuf> {
    let mut dir =
        dirs::config_dir().ok_or_else(|| anyhow!("could not resolve config directory"))?;
    dir.push(APP_NAME);
    Ok(dir)
}

fn config_path() -> Result<PathBuf> {
    let mut path = config_dir()?;
    path.push("config.toml");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn deserializing_partial_config_uses_defaults_for_missing_fields() {
        let raw = r#"
aliases = ["work"]

[fingerprints]
work = "abc123"
"#;

        let cfg: Config = toml::from_str(raw).expect("config should deserialize");

        assert_eq!(cfg.aliases, vec!["work"]);
        assert_eq!(cfg.last_used_alias, None);
        assert!(cfg.notifications.enabled);
        assert!(cfg.notifications.only_when_no_tty);
        assert!(cfg.notifications.only_on_implicit_cycle);
    }

    #[test]
    fn config_round_trip_preserves_metadata() {
        let original = Config {
            aliases: vec!["work".into(), "personal".into()],
            fingerprints: [
                ("work".to_string(), "1111222233334444".to_string()),
                ("personal".to_string(), "aaaabbbbccccdddd".to_string()),
            ]
            .into_iter()
            .collect(),
            notifications: Default::default(),
            last_used_alias: Some("personal".into()),
        };

        let serialized = toml::to_string(&original).expect("serialize config");
        let parsed: Config = toml::from_str(&serialized).expect("deserialize config");

        assert_eq!(parsed.aliases, original.aliases);
        assert_eq!(parsed.last_used_alias, original.last_used_alias);
        assert_eq!(parsed.fingerprints, original.fingerprints);
        assert_eq!(
            parsed.notifications.only_on_implicit_cycle,
            original.notifications.only_on_implicit_cycle
        );
    }
}
