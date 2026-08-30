<div align="center">

# grok-api

**CLI, MCP server, and local HTTP API for Grok Imagine, web search, and X search.**

[![CI](https://github.com/AkaraChen/grok-api/actions/workflows/ci.yml/badge.svg)](https://github.com/AkaraChen/grok-api/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AkaraChen/grok-api)](https://github.com/AkaraChen/grok-api/releases/latest)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org)

One binary. Same command shape everywhere:

```text
grok-api <resource> <command> [flags]
```

[Install](#install) · [Quick start](#quick-start) · [Usage](#usage) · [MCP](#mcp) · [HTTP](#http)

</div>

---

Generate and edit images, start and download videos, search the web or X — then expose the same resources over MCP or a local OpenAPI server. Credentials live in `~/.grok-api` so this CLI never writes `~/.grok` and never collides with official `grok`.

## Features

| Surface | What you get |
| --- | --- |
| **Images** | Text-to-image and multi-image edits via Grok Imagine |
| **Video** | Text-to-video and image-to-video, with poll, download, and TTS voices |
| **Search** | Official `web_search` and `x_search` |
| **MCP** | stdio for Cursor / Claude Desktop, or streamable HTTP |
| **HTTP** | Same resources on localhost, with Swagger UI at `/docs` |

## Install

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/AkaraChen/grok-api/main/install.sh | bash
```

Installs to `~/.local/bin`. Override with `GROK_API_INSTALL_DIR`.

### Windows

```powershell
irm https://raw.githubusercontent.com/AkaraChen/grok-api/main/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\grok-api` and adds that directory to your user `PATH`.

### Other options

```bash
# Scoop
scoop install https://github.com/AkaraChen/grok-api/releases/latest/download/grok-api.json

# From this repo
cargo install --path .
```

Prebuilt archives for Linux, macOS, and Windows (`x86_64` and `aarch64`) are on the [latest release](https://github.com/AkaraChen/grok-api/releases/latest).

<details>
<summary>Installer notes</summary>

The installer asks GitHub for the latest release. If `gh` is installed and logged in, that OAuth token is used automatically. You can also set `GITHUB_TOKEN` or `GH_TOKEN`. Without a token, unauthenticated GitHub API limits apply; if you hit them, pass a token or download the archive from the [releases page](https://github.com/AkaraChen/grok-api/releases/latest).

Winget cannot install a GitHub release URL directly — it needs a package in [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) — so it is not wired up here.

</details>

## Quick start

```bash
# Reuse an existing official Grok CLI login
grok-api auth login --from-grok-cli

# Or sign in into ~/.grok-api
grok-api auth login --oauth
grok-api auth login --device-auth
grok-api auth login --api-key xai-...

# Generate an image
grok-api image generate --prompt "A cat in a spacesuit on Mars" --aspect-ratio 16:9
```

`--api-key` and `XAI_API_KEY` override stored credentials. `auth logout --yes` deletes only `~/.grok-api/auth.json`. It never touches `~/.grok`.

## Usage

```text
Resources:
  auth       login, status, refresh, logout
  image      generate, model list
  video      generate, task get, download, voice list
  search     query, x
  mcp        stdio, http
  http       OpenAPI at /docs
  config     show, set
```

Add `--help` after any command for flags and defaults.

### Images

```bash
grok-api image generate --prompt "A cat in a spacesuit on Mars" --aspect-ratio 16:9
grok-api image generate --prompt "Logo design" --n 3 --out-dir ./generated/
grok-api image generate --prompt "A cat" --out /tmp/cat.jpg
grok-api image generate --prompt "combine these" --image subject.png --image style.png
grok-api image model list
```

### Video

```bash
grok-api video generate --prompt "Ocean waves at sunset." --download sunset.mp4
grok-api video generate --prompt "A robot painting." --async --quiet
grok-api video task get --task-id <request_id>
grok-api video download --file-id <request_id> --out out.mp4
grok-api video voice list
```

### Search

```bash
grok-api search query --query "xAI grok imagine API"
grok-api search query --query "tokio spawn" --allowed-domain docs.rs --allowed-domain tokio.rs
grok-api search x --query "What are people saying about xAI on X?"
grok-api search x --query "xAI status" --allowed-handle elonmusk --from-date 2025-10-01 --to-date 2025-10-10
```

### MCP

Same tools as the CLI. Auth is the process credentials (`XAI_API_KEY`, `~/.grok-api`, or `--from-grok-cli`).

```bash
# Cursor / Claude Desktop (stdio — default)
grok-api mcp

# Streamable HTTP
grok-api mcp http --bind 127.0.0.1:3920
```

### HTTP

Same resources as the CLI and MCP. Default bind is `127.0.0.1:8080`.

```bash
grok-api http
grok-api http --bind 127.0.0.1:8080
```

OpenAPI UI is `/docs`. Spec is `/api-docs/openapi.json`.

| Method | Path | CLI |
| --- | --- | --- |
| `GET` | `/auth/status` | `auth status` |
| `GET` | `/config` | `config show` |
| `POST` | `/image/generate` | `image generate` |
| `GET` | `/image/models` | `image model list` |
| `POST` | `/video/generate` | `video generate` |
| `GET` | `/video/tasks/{task_id}` | `video task get` |
| `POST` | `/video/download` | `video download` |
| `GET` | `/video/voices` | `video voice list` |
| `POST` | `/search/query` | `search query` |
| `POST` | `/search/x` | `search x` |

### Config

Stored under `~/.grok-api`.

```bash
grok-api config show
grok-api config set --key default_search_model --value grok-4.6
```

## Global flags

| Flag | Meaning |
| --- | --- |
| `--api-key` | Override all other auth (`XAI_API_KEY`) |
| `--from-grok-cli` | Read `~/.grok/auth.json` for this invocation |
| `--output json` | Machine-readable output |
| `--quiet` | Payload only |
| `--non-interactive` | No prompts (CI / agents) |
| `--dry-run` | Print the request without calling the API |

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

Working on the crate itself? See [AGENTS.md](AGENTS.md).
