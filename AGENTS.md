# Repository guidelines

## Project overview

- Ferris Lab is a Rust-based factory for building CLI utilities via multiple cooperating agents.
- The core agent runtime lives in `src/`, with the CLI entrypoint in `src/main.rs` and shared logic in `src/lib.rs`.
- Agents communicate over WebSockets (Axum + tokio-tungstenite) and can coordinate via Docker or local `cargo run`.
- Scripts for multi-agent orchestration live in `scripts/`, and integration coverage is in `tests/peer_communication.rs`.

## Required verifications

- After code changes (not documentation-only), run `cargo fmt --check` and `cargo test` before signaling readiness.
- If changes touch agent networking, WebSockets, or peer coordination, also run `cargo test --test peer_communication`.
- Fix failures locally and rerun the checks until clean.

## Git command restrictions

- Do not run git commands that stage, commit, amend, stash, or rewrite history (`git add`, `git commit`, `git reset`, etc.).
- Read-only inspection commands like `git status` or `git diff` are allowed when needed for context.
