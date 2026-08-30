#!/usr/bin/env bash
# Download the latest grok-api release for this machine (Linux / macOS / Git Bash).
set -euo pipefail

REPO="${GROK_API_REPO:-AkaraChen/grok-api}"
GITHUB_API="${GROK_API_GITHUB_API:-https://api.github.com}"
GITHUB_API="${GITHUB_API%/}"
BIN_NAME="grok-api"
INSTALL_DIR="${GROK_API_INSTALL_DIR:-${HOME}/.local/bin}"
RELEASES_PAGE="https://github.com/${REPO}/releases/latest"

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

print_rate_limit_help() {
  cat >&2 <<EOF
error: GitHub API rate limit exceeded.

This installer lists the latest release via:
  ${GITHUB_API}/repos/${REPO}/releases/latest

Unauthenticated requests are tightly limited. You can:

  1. export GITHUB_TOKEN=...          # or GH_TOKEN
  2. install GitHub CLI and run: gh auth login
     (this script uses that OAuth token automatically when gh is logged in)
  3. download the archive for your machine from:
     ${RELEASES_PAGE}
EOF
}

print_no_release_help() {
  cat >&2 <<EOF
error: no GitHub release found for ${REPO}.

Publish binaries by pushing a version tag (for example v0.1.0), or install from source:

  cargo install --path .

Manual downloads (once a release exists):
  ${RELEASES_PAGE}
EOF
}

resolve_token() {
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    printf '%s' "${GITHUB_TOKEN}"
    return 0
  fi
  if [[ -n "${GH_TOKEN:-}" ]]; then
    printf '%s' "${GH_TOKEN}"
    return 0
  fi
  if command -v gh >/dev/null 2>&1; then
    local token
    if token="$(gh auth token 2>/dev/null)" && [[ -n "${token}" ]]; then
      printf '%s' "${token}"
      return 0
    fi
  fi
  return 0
}

detect_target() {
  if [[ -n "${GROK_API_TARGET:-}" ]]; then
    printf '%s' "${GROK_API_TARGET}"
    return 0
  fi

  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}" in
    Linux)
      case "${arch}" in
        x86_64 | amd64) printf 'x86_64-unknown-linux-gnu' ;;
        aarch64 | arm64) printf 'aarch64-unknown-linux-gnu' ;;
        *) die "unsupported Linux architecture: ${arch}. Download from ${RELEASES_PAGE}" ;;
      esac
      ;;
    Darwin)
      case "${arch}" in
        x86_64) printf 'x86_64-apple-darwin' ;;
        arm64 | aarch64) printf 'aarch64-apple-darwin' ;;
        *) die "unsupported macOS architecture: ${arch}. Download from ${RELEASES_PAGE}" ;;
      esac
      ;;
    MINGW* | MSYS* | CYGWIN* | Windows_NT)
      case "${arch}" in
        x86_64 | amd64) printf 'x86_64-pc-windows-msvc' ;;
        aarch64 | arm64) printf 'aarch64-pc-windows-msvc' ;;
        *) die "unsupported Windows architecture: ${arch}. Download from ${RELEASES_PAGE}" ;;
      esac
      ;;
    *)
      die "unsupported OS: ${os}. Use install.ps1 on Windows, or download from ${RELEASES_PAGE}"
      ;;
  esac
}

archive_name() {
  local target="$1"
  if [[ "${target}" == *windows* ]]; then
    printf '%s-%s.zip' "${BIN_NAME}" "${target}"
  else
    printf '%s-%s.tar.gz' "${BIN_NAME}" "${target}"
  fi
}

json_release_fields() {
  local json_file="$1"
  local want_name="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 - "${json_file}" "${want_name}" <<'PY'
import json, sys
path, want = sys.argv[1], sys.argv[2]
with open(path, encoding="utf-8") as f:
    data = json.load(f)
tag = data.get("tag_name") or ""
html = data.get("html_url") or ""
url = sums = ""
for asset in data.get("assets") or []:
    name = asset.get("name") or ""
    download = asset.get("browser_download_url") or ""
    if name == want:
        url = download
    if name in ("SHA256SUMS", "sha256sums.txt"):
        sums = download
print(tag)
print(html)
print(url)
print(sums)
PY
    return 0
  fi
  if command -v jq >/dev/null 2>&1; then
    local tag html url sums
    tag="$(jq -r '.tag_name // empty' "${json_file}")"
    html="$(jq -r '.html_url // empty' "${json_file}")"
    url="$(jq -r --arg n "${want_name}" '.assets[]? | select(.name == $n) | .browser_download_url' "${json_file}")"
    sums="$(jq -r '.assets[]? | select(.name == "SHA256SUMS" or .name == "sha256sums.txt") | .browser_download_url' "${json_file}" | head -n 1)"
    printf '%s\n%s\n%s\n%s\n' "${tag}" "${html}" "${url}" "${sums}"
    return 0
  fi
  die "need python3 or jq to parse the GitHub release JSON"
}

github_curl() {
  local out_file="$1"
  local hdr_file="$2"
  local url="$3"
  shift 3
  local args=(
    -sS
    -D "${hdr_file}"
    -o "${out_file}"
    -w '%{http_code}'
    -H 'Accept: application/vnd.github+json'
    -H 'X-GitHub-Api-Version: 2022-11-28'
    -H 'User-Agent: grok-api-installer'
  )
  if [[ -n "${TOKEN}" ]]; then
    args+=(-H "Authorization: Bearer ${TOKEN}")
  fi
  curl "${args[@]}" "$@" "${url}"
}

file_sha256() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${path}" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${path}" | awk '{print $1}'
  else
    return 1
  fi
}

extract_expected_sha() {
  local sums_file="$1"
  local name="$2"
  awk -v n="${name}" '
    $NF == n || $NF == ("*" n) || $NF == ("./" n) { print $1; exit }
  ' "${sums_file}"
}

on_path() {
  local dir="$1"
  case ":${PATH}:" in
    *":${dir}:"*) return 0 ;;
    *) return 1 ;;
  esac
}

command -v curl >/dev/null 2>&1 || die "curl is required"

TOKEN="$(resolve_token || true)"
TARGET="$(detect_target)"
ARCHIVE="$(archive_name "${TARGET}")"
if [[ -n "${GROK_API_VERSION:-}" ]]; then
  API_URL="${GITHUB_API}/repos/${REPO}/releases/tags/${GROK_API_VERSION}"
else
  API_URL="${GITHUB_API}/repos/${REPO}/releases/latest"
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "${WORKDIR}"' EXIT
BODY="${WORKDIR}/release.json"
HDRS="${WORKDIR}/headers.txt"

CODE="$(github_curl "${BODY}" "${HDRS}" "${API_URL}" || true)"
if [[ -z "${CODE}" || "${CODE}" == "000" ]]; then
  die "failed to reach ${API_URL}"
fi

if [[ "${CODE}" == "404" ]]; then
  print_no_release_help
  exit 1
fi

REMAINING="$(grep -i '^x-ratelimit-remaining:' "${HDRS}" | awk '{print $2}' | tr -d '\r' || true)"
if [[ "${CODE}" == "429" || "${CODE}" == "403" ]]; then
  if [[ "${REMAINING}" == "0" ]] || grep -qi 'rate limit' "${BODY}"; then
    print_rate_limit_help
    exit 1
  fi
fi

if [[ "${CODE}" != "200" ]]; then
  warn "GitHub API returned HTTP ${CODE}"
  if [[ -s "${BODY}" ]]; then
    cat "${BODY}" >&2
    echo >&2
  fi
  echo "If this is a rate limit or auth problem, set GITHUB_TOKEN / run gh auth login, or download from ${RELEASES_PAGE}" >&2
  exit 1
fi

FIELDS_FILE="${WORKDIR}/fields.txt"
json_release_fields "${BODY}" "${ARCHIVE}" >"${FIELDS_FILE}"
TAG="$(sed -n '1p' "${FIELDS_FILE}")"
HTML_URL="$(sed -n '2p' "${FIELDS_FILE}")"
ASSET_URL="$(sed -n '3p' "${FIELDS_FILE}")"
SUMS_URL="$(sed -n '4p' "${FIELDS_FILE}")"
if [[ -n "${HTML_URL}" ]]; then
  RELEASES_PAGE="${HTML_URL}"
fi

if [[ -z "${ASSET_URL}" ]]; then
  die "latest release ${TAG:-unknown} has no asset ${ARCHIVE}. Download from ${RELEASES_PAGE}"
fi

ASSET_PATH="${WORKDIR}/${ARCHIVE}"
DL_HDRS="${WORKDIR}/dl-headers.txt"
DL_CODE="$(github_curl "${ASSET_PATH}" "${DL_HDRS}" "${ASSET_URL}" -L || true)"
if [[ "${DL_CODE}" != "200" ]]; then
  die "failed to download ${ARCHIVE} (HTTP ${DL_CODE:-000}). Try ${RELEASES_PAGE}"
fi

if [[ -n "${SUMS_URL}" ]]; then
  SUMS_PATH="${WORKDIR}/SHA256SUMS"
  SUMS_CODE="$(github_curl "${SUMS_PATH}" "${WORKDIR}/sums-headers.txt" "${SUMS_URL}" -L || true)"
  if [[ "${SUMS_CODE}" == "200" ]]; then
    EXPECTED="$(extract_expected_sha "${SUMS_PATH}" "${ARCHIVE}")"
    ACTUAL="$(file_sha256 "${ASSET_PATH}" || true)"
    if [[ -z "${ACTUAL}" ]]; then
      warn "sha256sum/shasum not found; skipping checksum verification"
    elif [[ -z "${EXPECTED}" ]]; then
      warn "SHA256SUMS has no entry for ${ARCHIVE}"
    elif [[ "${EXPECTED}" != "${ACTUAL}" ]]; then
      die "checksum mismatch for ${ARCHIVE}"
    fi
  else
    warn "could not download SHA256SUMS (HTTP ${SUMS_CODE})"
  fi
fi

EXTRACT="${WORKDIR}/extract"
mkdir -p "${EXTRACT}"
if [[ "${ARCHIVE}" == *.zip ]]; then
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "${ASSET_PATH}" -d "${EXTRACT}"
  elif command -v python3 >/dev/null 2>&1; then
    python3 - "${ASSET_PATH}" "${EXTRACT}" <<'PY'
import sys, zipfile
zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])
PY
  else
    die "need unzip or python3 to extract ${ARCHIVE}"
  fi
else
  tar -xzf "${ASSET_PATH}" -C "${EXTRACT}"
fi

SRC=""
if [[ -f "${EXTRACT}/${BIN_NAME}" ]]; then
  SRC="${EXTRACT}/${BIN_NAME}"
elif [[ -f "${EXTRACT}/${BIN_NAME}.exe" ]]; then
  SRC="${EXTRACT}/${BIN_NAME}.exe"
else
  SRC="$(find "${EXTRACT}" -type f \( -name "${BIN_NAME}" -o -name "${BIN_NAME}.exe" \) | head -n 1 || true)"
fi
[[ -n "${SRC}" ]] || die "archive ${ARCHIVE} did not contain ${BIN_NAME}"

DEST_NAME="$(basename "${SRC}")"
mkdir -p "${INSTALL_DIR}"
cp "${SRC}" "${INSTALL_DIR}/${DEST_NAME}"
chmod 755 "${INSTALL_DIR}/${DEST_NAME}"

printf 'Installed %s %s to %s\n' "${BIN_NAME}" "${TAG:-}" "${INSTALL_DIR}/${DEST_NAME}"
if ! on_path "${INSTALL_DIR}"; then
  printf 'Add this directory to your PATH:\n  export PATH="%s:$PATH"\n' "${INSTALL_DIR}"
fi
