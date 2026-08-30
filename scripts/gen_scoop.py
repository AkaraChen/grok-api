#!/usr/bin/env python3
"""Generate a Scoop manifest for a grok-api GitHub release."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--repo", default="AkaraChen/grok-api")
    parser.add_argument("--x64-hash", required=True)
    parser.add_argument("--arm64-hash", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    version = args.version.lstrip("v")
    base = f"https://github.com/{args.repo}/releases/download/v{version}"
    manifest = {
        "version": version,
        "description": "CLI for the Grok / xAI API",
        "homepage": f"https://github.com/{args.repo}",
        "license": "Apache-2.0",
        "architecture": {
            "64bit": {
                "url": f"{base}/grok-api-x86_64-pc-windows-msvc.zip",
                "hash": args.x64_hash.lower(),
            },
            "arm64": {
                "url": f"{base}/grok-api-aarch64-pc-windows-msvc.zip",
                "hash": args.arm64_hash.lower(),
            },
        },
        "bin": "grok-api.exe",
        "checkver": "github",
        "autoupdate": {
            "architecture": {
                "64bit": {
                    "url": f"https://github.com/{args.repo}/releases/download/v$version/grok-api-x86_64-pc-windows-msvc.zip"
                },
                "arm64": {
                    "url": f"https://github.com/{args.repo}/releases/download/v$version/grok-api-aarch64-pc-windows-msvc.zip"
                },
            }
        },
    }
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(manifest, indent=4) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
