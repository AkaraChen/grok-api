# grok-media-cli

CLI for Grok Imagine image and video generation. The command surface matches `mmx`:

```text
Usage: grok-media <resource> <command> [flags]
```

It pins official [xai-org/grok-build](https://github.com/xai-org/grok-build) crates (`xai-dirs`, `xai-grok-auth`, `xai-grok-env`, `xai-grok-paths`) and calls the same Imagine HTTP API the official tools use. Login is official `grok login` with `GROK_HOME` pointed at `~/.grok-media` instead of `~/.grok`.

`xai-grok-tools` is not a compile-time dependency: its `tools-api` build.rs requires the monorepo's vendored `protoc`/`dotslash`.

## Auth

Two credential sources:

1. **Read the official Grok CLI key** from `~/.grok/auth.json`:

   ```bash
   grok-media auth login --from-grok-cli
   grok-media --from-grok-cli image generate --prompt "a cat"
   ```

2. **Same login as Grok CLI**, stored under `~/.grok-media`:

   ```bash
   grok-media auth login --oauth
   grok-media auth login --device-auth
   grok-media auth login --api-key xai-...
   ```

`--api-key` and `XAI_API_KEY` win over stored credentials. `auth logout --yes` only deletes `~/.grok-media/auth.json`. It never touches `~/.grok`.

## Examples

```bash
grok-media image generate --prompt "A cat in a spacesuit on Mars" --aspect-ratio 16:9
grok-media image generate --prompt "Logo design" --n 3 --out-dir ./generated/
grok-media image generate --prompt "A cat" --out /tmp/cat.jpg
grok-media video generate --prompt "Ocean waves at sunset." --download sunset.mp4
grok-media video generate --prompt "A robot painting." --async --quiet
grok-media video task get --task-id <request_id>
grok-media config show
```

## Install

```bash
cargo install --path .
```
