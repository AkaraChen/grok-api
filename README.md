<div align="center">

# grok-api

**Bring Grok Imagine to your terminal.**

Generate images and videos, search the web and X, and connect your agents through MCP or HTTP.

[![CI](https://github.com/AkaraChen/grok-api/actions/workflows/ci.yml/badge.svg)](https://github.com/AkaraChen/grok-api/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AkaraChen/grok-api?color=168a91)](https://github.com/AkaraChen/grok-api/releases/latest)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-168a91)](https://www.apache.org/licenses/LICENSE-2.0)

[Install](#install) · [Quick start](#quick-start) · [Examples](#usage) · [MCP](#mcp) · [HTTP](#http)

<img src="docs/assets/imagine-rover.jpg" alt="Generated with grok-api: a white research rover crosses a black volcanic plain beneath a turquoise glacial arch." width="960">

*One prompt. One image. Made with grok-api.* [See the exact command →](docs/showcase.md)

</div>

## From a prompt to a file

```bash
grok-api image generate \
  --prompt "A white research rover on a volcanic plain beside turquoise ice" \
  --aspect-ratio 2:1 --out ./rover.jpg
```

One binary, three ways to work: **CLI · MCP · HTTP**. The command shape stays simple:
`grok-api <resource> <command> [flags]`.

| Create | Discover | Connect |
| --- | --- | --- |
| Generate and edit images, including multi-image edits | Search the web with domain filters | Run an MCP server over stdio or HTTP |
| Generate videos from text or an image; poll and download | Search X with handle and date filters | Serve a local HTTP API with Swagger UI |

## Install

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/AkaraChen/grok-api/main/install.sh | bash
```

Requires `curl`, `tar`, and either `python3` or `jq`. Installs to `~/.local/bin`;
add it to `PATH` if needed. Override the destination with `GROK_API_INSTALL_DIR`.

### Windows

```powershell
irm https://raw.githubusercontent.com/AkaraChen/grok-api/main/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\grok-api` and adds that directory to your user `PATH`.

### Other options

```bash
# Scoop
scoop install https://github.com/AkaraChen/grok-api/releases/latest/download/grok-api.json

# From source (requires Git and the stable Rust toolchain)
git clone https://github.com/AkaraChen/grok-api.git
cd grok-api
cargo install --path .
```

Prebuilt archives for Linux, macOS, and Windows (`x86_64` and `aarch64`) are on the [latest release](https://github.com/AkaraChen/grok-api/releases/latest).

<details>
<summary>Installer notes</summary>

The installer asks GitHub for the latest release. If `gh` is installed and logged in, that OAuth token is used automatically. You can also set `GITHUB_TOKEN` or `GH_TOKEN`. Without a token, unauthenticated GitHub API limits apply; if you hit them, pass a token or download the archive from the [releases page](https://github.com/AkaraChen/grok-api/releases/latest).

Winget cannot install a GitHub release URL directly — it needs a package in [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) — so it is not wired up here.

</details>

## Quick start

### 1. Authenticate

Choose **one** method. With an existing official Grok CLI login:

```bash
grok-api auth login --from-grok-cli
```

Or save your own API key (replace the placeholder):

```bash
grok-api auth login --api-key "YOUR_XAI_API_KEY"
```

### 2. Generate and save

```bash
grok-api image generate \
  --prompt "A white research rover on a volcanic plain beside turquoise ice" \
  --aspect-ratio 2:1 --out ./rover.jpg
```

`--out` saves one image to the path you choose. Use `--out-dir` for a batch.
Without either flag, the default URL response is printed rather than saved.
The cover uses a longer prompt: [reproduce the showcase](docs/showcase.md).

### 3. Make it your own

```bash
# Generate three variations
grok-api image generate --prompt "A tiny lunar greenhouse" --n 3 --out-dir ./generated/

# Inspect generation without submitting it
grok-api image generate --prompt "A tiny lunar greenhouse" --dry-run

# Discover available image models
grok-api image model list
```

## Authentication

OAuth and device login require the official `grok` executable on your `PATH`
(see [Grok Build](https://github.com/xai-org/grok-build)). Choose the flow you need:

```bash
grok-api auth login --oauth
grok-api auth login --device-auth
grok-api auth status
```

These login flows write to `~/.grok-api`. `--from-grok-cli` reads the existing
login in `~/.grok`; this CLI does not write to that directory.
An existing official login is also used as a fallback if the selected local store is empty.

For scripts, set `XAI_API_KEY` or pass `--api-key "YOUR_XAI_API_KEY"` for one invocation.
CLI flags / `XAI_API_KEY` take precedence over saved credentials in the CLI.
`grok-api auth logout --yes` deletes only `~/.grok-api/auth.json`; it leaves the
official login, environment variables, and config intact.

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
grok-api image generate --prompt "A cat" --out ./cat.jpg
grok-api image generate --prompt "combine these" --image subject.png --image style.png
grok-api image model list
```

### Video

Generate a clip and save it locally, or return a task ID for your own polling loop.

```bash
grok-api video generate --prompt "Ocean waves at sunset." --download sunset.mp4
grok-api video generate --prompt "A robot painting." --async --quiet
grok-api video task get --task-id "YOUR_REQUEST_ID"
grok-api video download --file-id "YOUR_REQUEST_ID" --out out.mp4
grok-api video voice list
```

Animate an image (use your own file, or the image from Quick start):

```bash
grok-api video generate \
  --image ./rover.jpg \
  --prompt "The rover moves slowly forward. A gentle camera push toward the ice." \
  --duration 5 --resolution 720p --download ./rover.mp4
```

The video commands are usage examples; no generated video is included in this README.
Replace `YOUR_REQUEST_ID` with the ID returned by `--async`. Download after the task
is complete; do not combine `--async` with `--download`.

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
| `--dry-run` | Preview supported operations, including image/video generation |

`--dry-run` does not cover every subcommand: `video task get` and
`video download` still contact the API. Use `--help` to inspect those commands.

## More documentation

- [Authentication](#authentication) — login options and credential storage
- [MCP](#mcp) / [HTTP](#http) — use the same resources in agents and applications
- [CLI help](#usage) — add `--help` to any command for all options
- [Showcase notes](docs/showcase.md) — exact image prompt, provenance, and design references
- [Contributor guide](AGENTS.md) — architecture, local checks, and releases

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

Working on the crate itself? See [AGENTS.md](AGENTS.md).
