---
name: grok-api
description: Run the grok-api CLI to generate or edit Grok Imagine images, start/poll/download Imagine videos, search the web via official web_search, serve MCP over stdio or streamable HTTP, and manage ~/.grok-api credentials. Use when the user asks to generate images or videos with Grok/xAI, animate a still, check a video task, download a clip, search with grok-api, start grok-api mcp, or log in with XAI_API_KEY / official grok auth. Do not use for the official grok TUI/coding agent, and do not invent Imagine or search fields the CLI does not expose.
metadata:
  title: grok-api
  icon: "🛰️"
---

# grok-api

Binary is `grok-api`, never `grok`. Shape is `<resource> <command> [flags]`.

Prefer an installed `grok-api`. From this repo use `cargo run --quiet --`. Confirm flags with `grok-api <resource> <command> --help` before inventing any.

## Agent defaults

Every non-interactive call:

```bash
grok-api --non-interactive --output json <resource> <command> [flags]
```

- `--output json` for parseable results. Text mode prints paths, task ids, or search prose.
- `--quiet` when only the payload matters.
- `--api-key` / `XAI_API_KEY` override stored credentials.
- `--from-grok-cli` reads `~/.grok/auth.json` for that invocation.
- `--dry-run` to show the request without calling the API.
- Pass HTTP error bodies through unchanged. Do not remap status codes.

## Auth

Check first:

```bash
grok-api --non-interactive --output json auth status
```

If unauthenticated, pick one (do not write `~/.grok`):

| Situation | Command |
| --- | --- |
| User already ran official `grok login` | `grok-api auth login --from-grok-cli` |
| Headless / CI key | `grok-api auth login --api-key "$XAI_API_KEY"` or export `XAI_API_KEY` |
| Interactive OAuth into `~/.grok-api` | `grok-api auth login --oauth` |
| Device code | `grok-api auth login --device-auth` |

`auth logout --yes` deletes only `~/.grok-api/auth.json`. Never delete `~/.grok`.

## Image

```bash
grok-api --non-interactive --output json image generate \
  --prompt "A cat in a spacesuit on Mars" \
  --aspect-ratio 16:9 \
  --out /tmp/cat.jpg
```

| Flag | Notes |
| --- | --- |
| `--prompt` | Required |
| `--image` | Local path or URL; switches to image edit |
| `--n` | 1–10. `--out` requires `--n 1` |
| `--out` / `--out-dir` / `--out-prefix` | Save files. Prefix default `image` |
| `--aspect-ratio` | `auto`, `1:1`, `16:9`, `9:16`, `4:3`, `3:4`, `3:2`, `2:3`, `2:1`, `1:2`, `21:9`, `19.5:9` |
| `--resolution` | `1k` (default), `2k` |
| `--quality` | `low`, `medium`, `auto`. `high` is rejected |
| `--response-format` | `url` (default) or `base64` (`b64_json` on the wire) |
| `--model` | Default `grok-imagine-image-2.0`. List with `image model list` |

List models: `grok-api --output json image model list`.

## Video

Video is async. Wait and save, or return a task id.

```bash
# Wait and download
grok-api --non-interactive --output json video generate \
  --prompt "Ocean waves at sunset." \
  --download sunset.mp4

# Agent/CI: return request_id immediately
grok-api --non-interactive --output json --quiet video generate \
  --prompt "A robot painting." \
  --async
grok-api --output json video task get --task-id <request_id>
grok-api video download --file-id <request_id> --out out.mp4
```

| Flag | Notes |
| --- | --- |
| `--prompt` | Required. Tag speakers as `<AUDIO_0>`, `<AUDIO_1>`, `<AUDIO_2>` |
| `--image` | Image-to-video start frame (path or URL) |
| `--reference-image` | Repeatable style/subject refs |
| `--voice` | Repeatable TTS `voice_id` (`eve`, `ara`, `leo`, `rex`, …). List with `video voice list` |
| `--duration` | Seconds |
| `--aspect-ratio` | `16:9`, `9:16`, `1:1`, `4:3`, `3:4` |
| `--resolution` | `480p` (API default if omitted), `720p`, `1080p` |
| `--async` / `--no-wait` | Same: print task id, do not poll |
| `--download` | Path written after the task completes |
| `--poll-interval` | Seconds (default 5) |

`--file-id` on `video download` accepts a request id or a completed video URL.

## Search

Official `web_search` tool via `POST /responses`. Default model `grok-4.6` (`GROK_WEB_SEARCH_MODEL` or `--model`).

```bash
grok-api --non-interactive --output json search query --query "xAI grok imagine API"
grok-api search query --query "tokio spawn" --allowed-domain docs.rs --allowed-domain tokio.rs
```

`--allowed-domain` and `--excluded-domain` are mutually exclusive on the API. Do not send both. JSON fields: `query`, `content`, `citations`, `allowed_domains`.

## MCP

Same tools as the CLI resources. Auth is the process credentials (`XAI_API_KEY`, `~/.grok-api`, or `--from-grok-cli`). stdio must not write to stdout except the protocol.

```bash
# Cursor / Claude Desktop spawn this
grok-api mcp
# or
grok-api mcp stdio

# Streamable HTTP at http://127.0.0.1:3920/mcp
grok-api mcp http --bind 127.0.0.1:3920
```

Tools: `image_generate`, `image_model_list`, `video_generate`, `video_task_get`, `video_download`, `video_voice_list`, `search_query`, `auth_status`, `config_show`. Field names match the CLI / official Imagine and web_search payloads.

## Config

Stored under `~/.grok-api`, never `~/.grok`.

```bash
grok-api --output json config show
grok-api config set --key default_search_model --value grok-4.6
```

Keys: `auth_source`, `base_url`, `output`, `timeout`, `api_key`, `default_image_model`, `default_video_model`, `default_search_model`.

## Do not

- Call official `grok` except as the login helper this CLI already wraps.
- Invent flags, JSON fields, quality values, model allowlists, or video status machines.
- Use `--quality high`, `--out` with `--n` other than 1, or both search domain lists.
- Treat this skill as permission to change grok-api internals; implementation rules live in repo `AGENTS.md`.
