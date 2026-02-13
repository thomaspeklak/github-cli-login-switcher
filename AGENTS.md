# AGENTS

## Development guardrails
- Keep files under **400 lines**. If a file approaches the limit, split logic into modules.
- Before considering work done, run:
  - `cargo fmt`
  - `cargo check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
- Prefer small, reviewable commits and focused changes.
