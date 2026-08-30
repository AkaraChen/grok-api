# grok-api

This crate is a **thin CLI glue layer** over official Grok CLI / [xai-org/grok-build](https://github.com/xai-org/grok-build). It is not a second Imagine implementation.

If a change needs new request shapes, retries, validation, defaults, or error mapping that grok-build does not already do: **do not write it here**. Pin or call the official crate, or spawn official `grok` with `GROK_HOME` pointed at `~/.grok-api`.

## Allowed

- clap surface (`<resource> <command> [flags]`) that forwards flags to official fields
- `~/.grok-api` home so this binary never writes `~/.grok` and never collides with official `grok`
- Reading the official `auth.json` shape; login is `GROK_HOME=~/.grok-api grok login` or `--from-grok-cli`
- Pinning `xai-dirs`, `xai-grok-auth`, `xai-grok-env`, `xai-grok-paths` at the rev in `Cargo.toml`
- Printing / saving the remote payload (`--out`, `--out-dir`, `--output json`)
- Passing HTTP error bodies through unchanged

## Forbidden

- Hand-written business logic, heuristics, or “helpful” client-side checks (quality enums, reference-image counts, voice caps, model allowlists)
- Inventing JSON fields, defaults, or status machines the official tools do not send
- Depending on `xai-grok-tools` (its `tools-api` build.rs needs the monorepo `protoc` / `dotslash`)
- Shipping a binary named `grok`

## Imagine HTTP

`src/infra/imagine.rs` exists only because `xai-grok-tools` cannot be compiled in this repo. Keep it a field-for-field copy of the official Imagine tools (`image_gen` / `imagine`, `image_to_video` / `imagine-video`). Do not grow it into an SDK.

Auth on session tokens: `Authorization: Bearer` plus `X-XAI-Token-Auth: xai-grok-cli` — same as grok-build.

## Commands

```bash
cargo test
cargo run -- image generate --help
```
