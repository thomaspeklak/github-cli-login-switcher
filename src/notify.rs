use std::{io, io::IsTerminal, process::Command};

use anyhow::{Context, Result, bail};

use crate::config::Config;

pub fn maybe_notify(cfg: &Config, implicit_cycle: bool, title: &str, body: &str) {
    if !should_notify(cfg, implicit_cycle) {
        return;
    }

    let _ = send_notification(title, body);
}

fn should_notify(cfg: &Config, implicit_cycle: bool) -> bool {
    if !cfg.notifications.enabled {
        return false;
    }

    if cfg.notifications.only_on_implicit_cycle && !implicit_cycle {
        return false;
    }

    if cfg.notifications.only_when_no_tty {
        let has_tty =
            io::stdin().is_terminal() || io::stdout().is_terminal() || io::stderr().is_terminal();
        if has_tty {
            return false;
        }
    }

    true
}

fn send_notification(title: &str, body: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let esc_title = title.replace('"', "\\\"");
        let esc_body = body.replace('"', "\\\"");
        let script = format!("display notification \"{esc_body}\" with title \"{esc_title}\"");

        let status = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status()
            .context("failed to run osascript")?;

        if !status.success() {
            bail!("osascript notification failed");
        }
        return Ok(());
    }

    #[cfg(target_os = "linux")]
    {
        let status = Command::new("notify-send")
            .arg(title)
            .arg(body)
            .status()
            .context("failed to run notify-send")?;

        if !status.success() {
            bail!("notify-send failed");
        }
        return Ok(());
    }

    #[allow(unreachable_code)]
    Ok(())
}
