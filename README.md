# gh-token-switch

<img alt="gh-token-switch logo" src="./logo.png" width="100" />

Switch between multiple GitHub CLI (`gh`) tokens using profile aliases, with secrets stored in your OS keychain.

`gh-token-switch` is useful when you use different fine-grained PATs for different orgs/repos and want fast switching without putting tokens in plaintext files.

## Why this exists

- Fine-grained PATs are intentionally narrow in scope, so one token is often not enough across work/personal/client contexts.
- Switching with raw `gh auth login` each time is repetitive when you bounce between contexts many times per day.
- This tool keeps switching fast while keeping secrets in OS keychain storage (not plaintext config files).

## What it does

- Stores token secrets in OS keychain (via `keyring`)
- Stores only non-secret metadata in config (`~/.config/gh-token-switch/config.toml`)
- Switches active `gh` auth by piping token to:
  - `gh auth login --hostname github.com --with-token`
- Supports cycling profiles with no alias argument
- Can show current managed alias by fingerprint matching
- Can optionally send desktop notifications for implicit cycle switches

---

## Requirements

- Linux or macOS
- `gh` installed and available in `PATH`
- A working system keychain backend:
  - macOS: Keychain Access
  - Linux: Secret Service-compatible keyring (common desktop setups)
- Optional for Linux notifications: `notify-send`

---

## Install / build

From source:

```bash
cargo build --release
```

Binary path:

```text
target/release/gh-token-switch
```

Optional: copy it somewhere in your `PATH`.

---

## Command reference (usage-focused)

## 1) Store or update a token

```bash
gh-token-switch set <alias>
```

Examples:

```bash
gh-token-switch set work
gh-token-switch set personal
```

You will be prompted securely (`GitHub token:`).

You can also pipe a token in non-interactive mode:

```bash
printf '%s' "$GITHUB_TOKEN" | gh-token-switch set ci
```

Notes:
- If alias exists, token is replaced (rotation/update flow)
- Alias is added to cycle order if new

## 2) Switch to a specific alias

```bash
gh-token-switch use <alias>
```

Example:

```bash
gh-token-switch use work
```

On success, prints the alias used.

## 3) Cycle to next alias

```bash
gh-token-switch use
# shortcut (same behavior)
gh-token-switch
```

Behavior:
- Requires at least 2 aliases
- Uses deterministic alias order (insertion order unless you rename/delete)
- If current alias is known, picks next
- If current `gh` token is unmanaged/unknown, picks first alias

## 4) Show current managed alias

```bash
gh-token-switch current
```

Output:
- alias name if current `gh` token matches stored fingerprint
- `unknown` if current token is not managed by this tool

## 5) List aliases

```bash
gh-token-switch list
```

Prints one alias per line.

## 6) Rename alias

```bash
gh-token-switch rename <old> <new>
```

Example:

```bash
gh-token-switch rename client-acme acme
```

Moves keychain secret + updates metadata.

## 7) Delete alias

```bash
gh-token-switch delete <alias>
```

Example:

```bash
gh-token-switch delete personal
```

Removes keychain entry and metadata.

---

## Typical workflows

## Initial setup

```bash
gh-token-switch set work
gh-token-switch set personal
gh-token-switch list
```

## Daily switching

Explicit:

```bash
gh-token-switch use work
```

Cycle:

```bash
gh-token-switch use
```

Check current managed alias:

```bash
gh-token-switch current
```

## Rotate a PAT

Just call `set` again with same alias:

```bash
gh-token-switch set work
```

---

## Notifications

Notifications are controlled by config:

- `notifications.enabled`
- `notifications.only_when_no_tty`
- `notifications.only_on_implicit_cycle`

Default intent:
- enabled
- only when running without interactive terminal
- only when running `use` in cycle mode (no explicit alias)

Notification delivery failures do **not** fail a successful token switch.

---

## Config file

Path:

```text
~/.config/gh-token-switch/config.toml
```

Contains non-secret metadata only, including:

- alias list/order
- token fingerprints (truncated SHA-256)
- notification settings
- `last_used_alias`

Example shape:

```toml
aliases = ["work", "personal"]
last_used_alias = "work"

[fingerprints]
work = "2e9d7f2d3a0bc123"
personal = "7bd89a2127ef9981"

[notifications]
enabled = true
only_when_no_tty = true
only_on_implicit_cycle = true
```

---

## Security notes

- Tokens are never stored in plaintext config
- Tokens are never printed
- Token switching passes token via stdin to `gh`
- Config stores only non-secret metadata

---

## Troubleshooting

## `gh auth ...` command failed

Ensure `gh` is installed and works:

```bash
gh --version
gh auth status
```

## `current` prints `unknown`

This means active `gh` token does not match any managed fingerprint.

Fix by setting token under alias and switching through this tool:

```bash
gh-token-switch set <alias>
gh-token-switch use <alias>
```

## Linux notification not shown

Check `notify-send` availability:

```bash
which notify-send
```

If missing, install your distro's libnotify package.
