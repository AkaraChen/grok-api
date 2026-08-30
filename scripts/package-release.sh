#!/usr/bin/env bash
# Package a release binary as grok-api-<target>.tar.gz or .zip
set -euo pipefail

TARGET="${1:?usage: package-release.sh <target> [bin] [out-dir]}"
BIN="${2:-grok-api}"
OUT_DIR="${3:-dist}"

EXT=""
if [[ "${TARGET}" == *windows* ]]; then
  EXT=".exe"
fi

SRC="target/${TARGET}/release/${BIN}${EXT}"
if [[ ! -f "${SRC}" ]]; then
  echo "error: missing ${SRC}" >&2
  exit 1
fi

if [[ "${TARGET}" != *windows* ]] && command -v strip >/dev/null 2>&1; then
  strip "${SRC}" || true
fi

mkdir -p "${OUT_DIR}"
STAGE="$(mktemp -d)"
trap 'rm -rf "${STAGE}"' EXIT
cp "${SRC}" "${STAGE}/${BIN}${EXT}"

if [[ "${TARGET}" == *windows* ]]; then
  ARCHIVE="${OUT_DIR}/${BIN}-${TARGET}.zip"
  python3 - "${STAGE}/${BIN}${EXT}" "${ARCHIVE}" "${BIN}${EXT}" <<'PY'
import sys, zipfile
src, dest, inner = sys.argv[1], sys.argv[2], sys.argv[3]
with zipfile.ZipFile(dest, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    zf.write(src, inner)
PY
else
  ARCHIVE="${OUT_DIR}/${BIN}-${TARGET}.tar.gz"
  tar -C "${STAGE}" -czf "${ARCHIVE}" "${BIN}${EXT}"
fi

echo "${ARCHIVE}"
