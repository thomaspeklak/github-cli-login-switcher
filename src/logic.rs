use anyhow::{Result, bail};
use sha2::{Digest, Sha256};

use crate::config::Config;

pub fn token_fingerprint(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    hex.chars().take(16).collect()
}

pub fn alias_for_token(cfg: &Config, token: &str) -> Option<String> {
    let fp = token_fingerprint(token.trim());
    cfg.fingerprints
        .iter()
        .find(|(_, value)| *value == &fp)
        .map(|(alias, _)| alias.clone())
}

pub fn choose_next_alias(aliases: &[String], current: Option<&str>) -> Result<String> {
    if aliases.len() < 2 {
        bail!("need at least 2 aliases to cycle; add more with 'set <alias>'");
    }

    match current {
        Some(alias) => {
            let idx = aliases.iter().position(|a| a == alias).unwrap_or(0);
            let next = (idx + 1) % aliases.len();
            Ok(aliases[next].clone())
        }
        None => Ok(aliases[0].clone()),
    }
}

pub fn apply_rename_metadata(cfg: &mut Config, old: &str, new: &str) {
    if let Some(idx) = cfg.aliases.iter().position(|a| a == old) {
        cfg.aliases[idx] = new.to_string();
    } else {
        cfg.aliases.push(new.to_string());
    }

    if let Some(fp) = cfg.fingerprints.remove(old) {
        cfg.fingerprints.insert(new.to_string(), fp);
    }

    if cfg.last_used_alias.as_deref() == Some(old) {
        cfg.last_used_alias = Some(new.to_string());
    }
}

pub fn apply_delete_metadata(cfg: &mut Config, alias: &str) {
    cfg.aliases.retain(|a| a != alias);
    cfg.fingerprints.remove(alias);
    if cfg.last_used_alias.as_deref() == Some(alias) {
        cfg.last_used_alias = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, NotificationConfig};

    use super::{
        alias_for_token, apply_delete_metadata, apply_rename_metadata, choose_next_alias,
        token_fingerprint,
    };

    #[test]
    fn fingerprint_is_stable_and_truncated() {
        let fp = token_fingerprint("abc");
        assert_eq!(fp, "ba7816bf8f01cfea");
        assert_eq!(fp.len(), 16);
    }

    #[test]
    fn alias_lookup_by_token_fingerprint_works() {
        let mut cfg = Config {
            aliases: vec!["work".into()],
            fingerprints: Default::default(),
            notifications: NotificationConfig::default(),
            last_used_alias: None,
        };
        cfg.fingerprints
            .insert("work".into(), token_fingerprint("secret-token"));

        assert_eq!(alias_for_token(&cfg, "secret-token"), Some("work".into()));
        assert_eq!(alias_for_token(&cfg, "other-token"), None);
    }

    #[test]
    fn choose_next_alias_cycles_and_handles_unknown() {
        let aliases = vec![
            "work".to_string(),
            "personal".to_string(),
            "acme".to_string(),
        ];

        assert_eq!(
            choose_next_alias(&aliases, Some("work")).unwrap(),
            "personal"
        );
        assert_eq!(choose_next_alias(&aliases, Some("acme")).unwrap(), "work");
        assert_eq!(choose_next_alias(&aliases, None).unwrap(), "work");
        assert_eq!(
            choose_next_alias(&aliases, Some("missing")).unwrap(),
            "personal"
        );
    }

    #[test]
    fn choose_next_alias_requires_two_profiles() {
        let aliases = vec!["work".to_string()];
        let err = choose_next_alias(&aliases, None).unwrap_err().to_string();
        assert!(err.contains("need at least 2 aliases"));
    }

    #[test]
    fn rename_updates_alias_fingerprint_and_last_used() {
        let mut cfg = Config {
            aliases: vec!["work".into(), "personal".into()],
            fingerprints: [
                ("work".to_string(), "aaaabbbbccccdddd".to_string()),
                ("personal".to_string(), "eeeeffff00001111".to_string()),
            ]
            .into_iter()
            .collect(),
            notifications: NotificationConfig::default(),
            last_used_alias: Some("work".into()),
        };

        apply_rename_metadata(&mut cfg, "work", "company");

        assert_eq!(cfg.aliases, vec!["company", "personal"]);
        assert_eq!(
            cfg.fingerprints.get("company").map(String::as_str),
            Some("aaaabbbbccccdddd")
        );
        assert!(!cfg.fingerprints.contains_key("work"));
        assert_eq!(cfg.last_used_alias.as_deref(), Some("company"));
    }

    #[test]
    fn delete_updates_aliases_fingerprints_and_last_used() {
        let mut cfg = Config {
            aliases: vec!["work".into(), "personal".into()],
            fingerprints: [
                ("work".to_string(), "aaaabbbbccccdddd".to_string()),
                ("personal".to_string(), "eeeeffff00001111".to_string()),
            ]
            .into_iter()
            .collect(),
            notifications: NotificationConfig::default(),
            last_used_alias: Some("personal".into()),
        };

        apply_delete_metadata(&mut cfg, "personal");

        assert_eq!(cfg.aliases, vec!["work"]);
        assert!(!cfg.fingerprints.contains_key("personal"));
        assert_eq!(cfg.last_used_alias, None);
    }
}
