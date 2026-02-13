mod config;
mod logic;
mod notify;

use std::{
    io::{self, IsTerminal, Read, Write},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};
use keyring::Entry;

use crate::config::{APP_NAME, Config, ensure_alias, load_config, save_config};
use crate::logic::{
    alias_for_token, apply_delete_metadata, apply_rename_metadata, choose_next_alias,
    token_fingerprint,
};
use crate::notify::maybe_notify;

const SERVICE: &str = "github-cli-login-switcher";

#[derive(Parser, Debug)]
#[command(name = APP_NAME, version, about = "Switch GitHub auth tokens by profile")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Store/update token for a profile alias
    Set { alias: String },
    /// Switch to a profile alias, or cycle when omitted
    Use { alias: Option<String> },
    /// Show current active managed alias
    Current,
    /// List profile aliases
    List,
    /// Rename an alias in keychain and config
    Rename { old: String, new: String },
    /// Delete an alias from keychain and config
    Delete { alias: String },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_config()?;

    match cli.command {
        Some(Commands::Set { alias }) => set_token(&alias, &mut config),
        Some(Commands::Use { alias }) => use_token(alias, &mut config),
        Some(Commands::Current) => current_alias(&config).map(|alias| {
            println!("{}", alias.unwrap_or_else(|| "unknown".to_string()));
        }),
        Some(Commands::List) => {
            for alias in &config.aliases {
                println!("{alias}");
            }
            Ok(())
        }
        Some(Commands::Rename { old, new }) => rename_alias(&old, &new, &mut config),
        Some(Commands::Delete { alias }) => delete_alias(&alias, &mut config),
        None => use_token(None, &mut config),
    }
}

fn entry(alias: &str) -> Result<Entry> {
    Entry::new(SERVICE, alias).context("failed to create keyring entry")
}

fn set_token(alias: &str, cfg: &mut Config) -> Result<()> {
    let token = read_token_from_user()?;
    if token.trim().is_empty() {
        bail!("token is empty");
    }

    entry(alias)?
        .set_password(token.trim())
        .with_context(|| format!("failed storing token for alias '{alias}'"))?;

    ensure_alias(cfg, alias);
    cfg.fingerprints
        .insert(alias.to_string(), token_fingerprint(token.trim()));
    save_config(cfg)?;

    println!("stored token for alias '{alias}'");
    Ok(())
}

fn use_token(alias_arg: Option<String>, cfg: &mut Config) -> Result<()> {
    let implicit_cycle = alias_arg.is_none();
    let target = match alias_arg {
        Some(alias) => alias,
        None => pick_next_alias(cfg)?,
    };

    let token = entry(&target)?
        .get_password()
        .with_context(|| format!("no token found for alias '{target}'"))?;

    let switched = switch_gh_token(&token);

    match switched {
        Ok(()) => {
            ensure_alias(cfg, &target);
            cfg.fingerprints
                .insert(target.clone(), token_fingerprint(&token));
            cfg.last_used_alias = Some(target.clone());
            save_config(cfg)?;

            maybe_notify(
                cfg,
                implicit_cycle,
                "GitHub token switched",
                &format!("Switched GitHub token: {target}"),
            );

            println!("{target}");
            Ok(())
        }
        Err(err) => {
            maybe_notify(
                cfg,
                implicit_cycle,
                "GitHub token switch failed",
                &format!("Failed switching to: {target}"),
            );
            Err(err)
        }
    }
}

fn current_alias(cfg: &Config) -> Result<Option<String>> {
    let Some(active_token) = gh_current_token().ok() else {
        return Ok(None);
    };

    Ok(alias_for_token(cfg, &active_token))
}

fn rename_alias(old: &str, new: &str, cfg: &mut Config) -> Result<()> {
    if old == new {
        bail!("old and new alias are identical");
    }

    if cfg.aliases.iter().any(|a| a == new) {
        bail!("alias '{new}' already exists");
    }

    let token = entry(old)?
        .get_password()
        .with_context(|| format!("no token found for alias '{old}'"))?;

    entry(new)?
        .set_password(&token)
        .with_context(|| format!("failed storing token for alias '{new}'"))?;

    entry(old)?
        .delete_credential()
        .with_context(|| format!("failed deleting old alias '{old}'"))?;

    apply_rename_metadata(cfg, old, new);
    save_config(cfg)?;
    println!("renamed '{old}' -> '{new}'");
    Ok(())
}

fn delete_alias(alias: &str, cfg: &mut Config) -> Result<()> {
    let _ = entry(alias)?.delete_credential();
    apply_delete_metadata(cfg, alias);
    save_config(cfg)?;
    println!("deleted '{alias}'");
    Ok(())
}

fn pick_next_alias(cfg: &Config) -> Result<String> {
    let current = current_alias(cfg)?;
    choose_next_alias(&cfg.aliases, current.as_deref())
}

fn gh_current_token() -> Result<String> {
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("failed to run 'gh auth token' (is gh installed?)")?;

    if !output.status.success() {
        bail!("'gh auth token' failed");
    }

    String::from_utf8(output.stdout).context("gh output was not utf-8")
}

fn switch_gh_token(token: &str) -> Result<()> {
    let mut child = Command::new("gh")
        .args(["auth", "login", "--hostname", "github.com", "--with-token"])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to run 'gh auth login' (is gh installed?)")?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("failed to open stdin for gh process"))?;
        stdin
            .write_all(token.as_bytes())
            .context("failed to pass token to gh")?;
    }

    let status = child.wait().context("failed waiting for gh process")?;
    if !status.success() {
        bail!("'gh auth login --with-token' failed");
    }
    Ok(())
}

fn read_token_from_user() -> Result<String> {
    if io::stdin().is_terminal() {
        return rpassword::prompt_password("GitHub token: ").context("failed to read token");
    }

    let mut token = String::new();
    io::stdin()
        .read_to_string(&mut token)
        .context("failed to read token from stdin")?;
    Ok(token.trim().to_string())
}
