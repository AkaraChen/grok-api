# grok-api

CLI for the Grok / xAI API. The command surface matches `mmx`:

```text
Usage: grok-api <resource> <command> [flags]
```

It pins official [xai-org/grok-build](https://github.com/xai-org/grok-build) crates (`xai-dirs`, `xai-grok-auth`, `xai-grok-env`, `xai-grok-paths`) and calls the same HTTP APIs the official tools use. Login is official `grok login` with `GROK_HOME` pointed at `~/.grok-api` instead of `~/.grok`.

`xai-grok-tools` is not a compile-time dependency: its `tools-api` build.rs requires the monorepo's vendored `protoc`/`dotslash`.

`grok-api http` serves the same CLI resources over Axum. OpenAPI (utoipa) is at `/docs`; the spec is `/api-docs/openapi.json`. Default bind is `127.0.0.1:8080`.

## Auth

Two credential sources:

1. **Read the official Grok CLI key** from `~/.grok/auth.json`:

   ```bash
   grok-api auth login --from-grok-cli
   grok-api --from-grok-cli image generate --prompt "a cat"
   ```

2. **Same login as Grok CLI**, stored under `~/.grok-api`:

   ```bash
   grok-api auth login --oauth
   grok-api auth login --device-auth
   grok-api auth login --api-key xai-...
   ```

`--api-key` and `XAI_API_KEY` win over stored credentials. `auth logout --yes` only deletes `~/.grok-api/auth.json`. It never touches `~/.grok`.

## Examples

```bash
grok-api image generate --prompt "A cat in a spacesuit on Mars" --aspect-ratio 16:9
grok-api image generate --prompt "Logo design" --n 3 --out-dir ./generated/
grok-api image generate --prompt "A cat" --out /tmp/cat.jpg
grok-api video generate --prompt "Ocean waves at sunset." --download sunset.mp4
grok-api video generate --prompt "A robot painting." --async --quiet
grok-api video task get --task-id <request_id>
grok-api search query --query "xAI grok imagine API"
grok-api search query --query "tokio spawn" --allowed-domain docs.rs --allowed-domain tokio.rs
grok-api mcp
grok-api mcp http --bind 127.0.0.1:3920
grok-api http
grok-api http --bind 127.0.0.1:8080
grok-api config show
```

## Install

```bash
cargo install --path .
```
