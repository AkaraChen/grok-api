#!/usr/bin/env python3
"""End-to-end tests for install.sh / install.ps1 against a mock GitHub API."""

from __future__ import annotations

import hashlib
import http.server
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INSTALL_SH = ROOT / "install.sh"
INSTALL_PS1 = ROOT / "install.ps1"
REPO = "AkaraChen/grok-api"
TAG = "v0.1.0"
LINUX_TARGET = "x86_64-unknown-linux-gnu"
WIN_TARGET = "x86_64-pc-windows-msvc"
LINUX_ARCHIVE = f"grok-api-{LINUX_TARGET}.tar.gz"
WIN_ARCHIVE = f"grok-api-{WIN_TARGET}.zip"
MARKER = "grok-api-e2e-ok"


class Failures(list):
    def check(self, name: str, ok: bool, detail: str = "") -> None:
        status = "PASS" if ok else "FAIL"
        extra = f" — {detail}" if detail else ""
        print(f"[{status}] {name}{extra}")
        if not ok:
            self.append(name)


class GitHubHandler(http.server.BaseHTTPRequestHandler):
    server_version = "grok-api-mock/1.0"

    def log_message(self, fmt: str, *args) -> None:
        return

    def _mode(self) -> str:
        return self.server.mode  # type: ignore[attr-defined]

    def _token(self) -> str | None:
        return self.server.expected_token  # type: ignore[attr-defined]

    def _fixtures(self) -> Path:
        return self.server.fixtures  # type: ignore[attr-defined]

    def _json(self, code: int, payload: dict, extra_headers: dict | None = None) -> None:
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        for key, value in (extra_headers or {}).items():
            self.send_header(key, value)
        self.end_headers()
        self.wfile.write(body)

    def _bytes(self, code: int, data: bytes, content_type: str = "application/octet-stream") -> None:
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _auth_ok(self) -> bool:
        expected = self._token()
        if not expected:
            return True
        header = self.headers.get("Authorization", "")
        return header == f"Bearer {expected}"

    def do_GET(self) -> None:
        path = self.path.split("?", 1)[0]
        mode = self._mode()
        if path in (
            f"/repos/{REPO}/releases/latest",
            f"/repos/{REPO}/releases/tags/{TAG}",
        ):
            if mode == "rate_limit":
                self._json(
                    403,
                    {"message": "API rate limit exceeded for 127.0.0.1"},
                    {"X-RateLimit-Remaining": "0", "X-RateLimit-Limit": "60"},
                )
                return
            if mode == "not_found":
                self._json(404, {"message": "Not Found"})
                return
            if not self._auth_ok():
                self._json(401, {"message": "Bad credentials"})
                return
            origin = self.server.origin  # type: ignore[attr-defined]
            assets = [
                {
                    "name": LINUX_ARCHIVE,
                    "browser_download_url": f"{origin}/download/{LINUX_ARCHIVE}",
                },
                {
                    "name": WIN_ARCHIVE,
                    "browser_download_url": f"{origin}/download/{WIN_ARCHIVE}",
                },
                {
                    "name": "SHA256SUMS",
                    "browser_download_url": f"{origin}/download/SHA256SUMS",
                },
            ]
            if mode == "missing_asset":
                assets = [assets[-1]]
            self._json(
                200,
                {
                    "tag_name": TAG,
                    "html_url": f"{origin}/releases/{TAG}",
                    "assets": assets,
                },
            )
            return

        if path.startswith("/download/"):
            if not self._auth_ok() and self._token():
                self._json(401, {"message": "Bad credentials"})
                return
            name = path.rsplit("/", 1)[-1]
            file_path = self._fixtures() / name
            if not file_path.is_file():
                self._json(404, {"message": f"missing {name}"})
                return
            self._bytes(200, file_path.read_bytes())
            return

        self._json(404, {"message": f"unhandled {path}"})


def start_server(fixtures: Path, mode: str, expected_token: str | None) -> tuple[http.server.HTTPServer, str]:
    server = http.server.HTTPServer(("127.0.0.1", 0), GitHubHandler)
    server.mode = mode
    server.expected_token = expected_token
    server.fixtures = fixtures
    host, port = server.server_address
    server.origin = f"http://{host}:{port}"
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, server.origin


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_binary(path: Path, name: str) -> None:
    path.write_text(f"#!/bin/sh\necho {MARKER}\n", encoding="utf-8")
    path.chmod(0o755)
    # Keep a sibling copy used only for zip inner name.
    _ = name


def build_fixtures(root: Path) -> Path:
    fixtures = root / "fixtures"
    fixtures.mkdir()
    linux_bin = root / "grok-api"
    write_binary(linux_bin, "grok-api")
    tarball = fixtures / LINUX_ARCHIVE
    with tarfile.open(tarball, "w:gz") as tar:
        tar.add(linux_bin, arcname="grok-api")

    win_bin = root / "grok-api.exe"
    win_bin.write_bytes(b"MZ-fake-pe " + MARKER.encode())
    zipped = fixtures / WIN_ARCHIVE
    with zipfile.ZipFile(zipped, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.write(win_bin, arcname="grok-api.exe")

    (fixtures / "SHA256SUMS").write_text(
        f"{sha256(tarball)}  {LINUX_ARCHIVE}\n{sha256(zipped)}  {WIN_ARCHIVE}\n",
        encoding="utf-8",
    )
    return fixtures


def run(
    command: list[str],
    env: dict[str, str],
    cwd: Path | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd or ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


def base_env(install_dir: Path, origin: str, extra: dict[str, str] | None = None) -> dict[str, str]:
    env = os.environ.copy()
    for key in ("GITHUB_TOKEN", "GH_TOKEN", "GROK_API_VERSION", "GROK_API_TARGET"):
        env.pop(key, None)
    env.update(
        {
            "GROK_API_REPO": REPO,
            "GROK_API_GITHUB_API": origin,
            "GROK_API_INSTALL_DIR": str(install_dir),
            "GROK_API_TARGET": LINUX_TARGET,
        }
    )
    if extra:
        env.update(extra)
    return env


def write_fake_gh(bin_dir: Path, token: str | None, fail: bool = False) -> None:
    path = bin_dir / "gh"
    if fail:
        path.write_text("#!/bin/sh\nexit 1\n", encoding="utf-8")
    else:
        path.write_text(
            "#!/bin/sh\n"
            'if [ "$1" = "auth" ] && [ "$2" = "token" ]; then\n'
            f"  printf '%s\\n' '{token}'\n"
            "  exit 0\n"
            "fi\n"
            "exit 1\n",
            encoding="utf-8",
        )
    path.chmod(0o755)


def without_real_gh(env: dict[str, str], fake_bin: Path | None = None) -> dict[str, str]:
    path_parts = [part for part in env.get("PATH", "").split(os.pathsep) if part]
    filtered = []
    for part in path_parts:
        gh = Path(part) / "gh"
        if gh.exists():
            continue
        filtered.append(part)
    if fake_bin is not None:
        filtered.insert(0, str(fake_bin))
    env = env.copy()
    env["PATH"] = os.pathsep.join(filtered)
    return env


def installed_ok(install_dir: Path) -> bool:
    binary = install_dir / "grok-api"
    if not binary.is_file() or not os.access(binary, os.X_OK):
        return False
    result = subprocess.run([str(binary)], text=True, capture_output=True, check=False)
    return result.returncode == 0 and MARKER in result.stdout


def main() -> int:
    failures = Failures()
    sh_syntax = run(["bash", "-n", str(INSTALL_SH)], os.environ.copy())
    failures.check("install.sh bash -n", sh_syntax.returncode == 0, sh_syntax.stderr.strip())

    with tempfile.TemporaryDirectory(prefix="grok-api-install-e2e-") as tmp_s:
        tmp = Path(tmp_s)
        fixtures = build_fixtures(tmp)

        # Happy path, no token, no real gh.
        install_dir = tmp / "ok"
        install_dir.mkdir()
        server, origin = start_server(fixtures, "ok", None)
        try:
            env = without_real_gh(base_env(install_dir, origin))
            result = run(["bash", str(INSTALL_SH)], env)
            failures.check(
                "install.sh happy path",
                result.returncode == 0 and installed_ok(install_dir),
                result.stderr + result.stdout,
            )
        finally:
            server.shutdown()

        # GITHUB_TOKEN is sent.
        install_dir = tmp / "token"
        install_dir.mkdir()
        server, origin = start_server(fixtures, "ok", "explicit-token")
        try:
            env = without_real_gh(base_env(install_dir, origin, {"GITHUB_TOKEN": "explicit-token"}))
            result = run(["bash", str(INSTALL_SH)], env)
            failures.check(
                "install.sh GITHUB_TOKEN",
                result.returncode == 0 and installed_ok(install_dir),
                result.stderr + result.stdout,
            )
        finally:
            server.shutdown()

        # gh auth token is used when logged in.
        install_dir = tmp / "gh"
        install_dir.mkdir()
        fake_bin = tmp / "fakegh"
        fake_bin.mkdir()
        write_fake_gh(fake_bin, "gh-oauth-token")
        server, origin = start_server(fixtures, "ok", "gh-oauth-token")
        try:
            env = without_real_gh(base_env(install_dir, origin), fake_bin)
            result = run(["bash", str(INSTALL_SH)], env)
            failures.check(
                "install.sh gh auth token",
                result.returncode == 0 and installed_ok(install_dir),
                result.stderr + result.stdout,
            )
        finally:
            server.shutdown()

        # gh present but not logged in still works unauthenticated.
        install_dir = tmp / "gh-fail"
        install_dir.mkdir()
        fake_fail = tmp / "fakegh-fail"
        fake_fail.mkdir()
        write_fake_gh(fake_fail, None, fail=True)
        server, origin = start_server(fixtures, "ok", None)
        try:
            env = without_real_gh(base_env(install_dir, origin), fake_fail)
            result = run(["bash", str(INSTALL_SH)], env)
            failures.check(
                "install.sh gh present but logged out",
                result.returncode == 0 and installed_ok(install_dir),
                result.stderr + result.stdout,
            )
        finally:
            server.shutdown()

        # Rate limit help.
        install_dir = tmp / "rl"
        install_dir.mkdir()
        server, origin = start_server(fixtures, "rate_limit", None)
        try:
            env = without_real_gh(base_env(install_dir, origin))
            result = run(["bash", str(INSTALL_SH)], env)
            text = result.stdout + result.stderr
            failures.check(
                "install.sh rate limit help",
                result.returncode != 0
                and "rate limit" in text.lower()
                and "GITHUB_TOKEN" in text
                and "gh auth login" in text
                and "releases/latest" in text,
                text,
            )
        finally:
            server.shutdown()

        # Missing release.
        install_dir = tmp / "none"
        install_dir.mkdir()
        server, origin = start_server(fixtures, "not_found", None)
        try:
            env = without_real_gh(base_env(install_dir, origin))
            result = run(["bash", str(INSTALL_SH)], env)
            text = result.stdout + result.stderr
            failures.check(
                "install.sh no release",
                result.returncode != 0 and "no GitHub release" in text and "releases/latest" in text,
                text,
            )
        finally:
            server.shutdown()

        # Missing platform asset.
        install_dir = tmp / "missing"
        install_dir.mkdir()
        server, origin = start_server(fixtures, "missing_asset", None)
        try:
            env = without_real_gh(base_env(install_dir, origin))
            result = run(["bash", str(INSTALL_SH)], env)
            text = result.stdout + result.stderr
            failures.check(
                "install.sh missing asset",
                result.returncode != 0 and LINUX_ARCHIVE in text,
                text,
            )
        finally:
            server.shutdown()

        pwsh = shutil.which("pwsh")
        if pwsh:
            ps_syntax = run(
                [
                    pwsh,
                    "-NoProfile",
                    "-Command",
                    f"$null = [System.Management.Automation.Language.Parser]::ParseFile('{INSTALL_PS1}', [ref]$null, [ref]$errs); if ($errs) {{ $errs | ForEach-Object {{ $_.ToString() }}; exit 1 }}",
                ],
                os.environ.copy(),
            )
            failures.check("install.ps1 parse", ps_syntax.returncode == 0, ps_syntax.stderr + ps_syntax.stdout)

            install_dir = tmp / "ps-ok"
            install_dir.mkdir()
            server, origin = start_server(fixtures, "ok", None)
            try:
                env = without_real_gh(
                    base_env(
                        install_dir,
                        origin,
                        {"GROK_API_TARGET": WIN_TARGET},
                    )
                )
                result = run(
                    [
                        pwsh,
                        "-NoProfile",
                        "-Command",
                        f"Get-Content -Raw -Path '{INSTALL_PS1}' | Invoke-Expression",
                    ],
                    env,
                )
                dest = install_dir / "grok-api.exe"
                failures.check(
                    "install.ps1 irm|iex happy path",
                    result.returncode == 0 and dest.is_file() and MARKER.encode() in dest.read_bytes(),
                    result.stderr + result.stdout,
                )
            finally:
                server.shutdown()

            install_dir = tmp / "ps-gh"
            install_dir.mkdir()
            server, origin = start_server(fixtures, "ok", "gh-oauth-token")
            try:
                env = without_real_gh(
                    base_env(install_dir, origin, {"GROK_API_TARGET": WIN_TARGET}),
                    fake_bin,
                )
                result = run([pwsh, "-NoProfile", "-File", str(INSTALL_PS1)], env)
                dest = install_dir / "grok-api.exe"
                failures.check(
                    "install.ps1 gh auth token",
                    result.returncode == 0 and dest.is_file(),
                    result.stderr + result.stdout,
                )
            finally:
                server.shutdown()

            install_dir = tmp / "ps-rl"
            install_dir.mkdir()
            server, origin = start_server(fixtures, "rate_limit", None)
            try:
                env = without_real_gh(base_env(install_dir, origin, {"GROK_API_TARGET": WIN_TARGET}))
                result = run([pwsh, "-NoProfile", "-File", str(INSTALL_PS1)], env)
                text = result.stdout + result.stderr
                failures.check(
                    "install.ps1 rate limit help",
                    result.returncode != 0
                    and "rate limit" in text.lower()
                    and "GITHUB_TOKEN" in text
                    and "gh auth login" in text
                    and "releases/latest" in text,
                    text,
                )
            finally:
                server.shutdown()
        else:
            print("[SKIP] pwsh not installed; install.ps1 tests skipped")

        scoop = tmp / "grok-api.json"
        gen = run(
            [
                sys.executable,
                str(ROOT / "scripts" / "gen_scoop.py"),
                "--version",
                "0.1.0",
                "--x64-hash",
                "aa" * 32,
                "--arm64-hash",
                "bb" * 32,
                "--out",
                str(scoop),
            ],
            os.environ.copy(),
        )
        if gen.returncode == 0:
            data = json.loads(scoop.read_text(encoding="utf-8"))
            failures.check(
                "gen_scoop.py",
                data["version"] == "0.1.0"
                and data["bin"] == "grok-api.exe"
                and "x86_64-pc-windows-msvc.zip" in data["architecture"]["64bit"]["url"],
            )
        else:
            failures.check("gen_scoop.py", False, gen.stderr)

        # Real GitHub API: this repo currently has no releases.
        real_dir = tmp / "real"
        real_dir.mkdir()
        real_env = os.environ.copy()
        real_env.update(
            {
                "GROK_API_REPO": REPO,
                "GROK_API_INSTALL_DIR": str(real_dir),
                "GROK_API_TARGET": LINUX_TARGET,
            }
        )
        real = run(["bash", str(INSTALL_SH)], real_env)
        text = real.stdout + real.stderr
        failures.check(
            "install.sh real GitHub API (no release yet)",
            real.returncode != 0
            and ("no GitHub release" in text or "rate limit" in text.lower())
            and "releases/latest" in text,
            text,
        )

    if failures:
        print(f"\n{len(failures)} test(s) failed: {', '.join(failures)}")
        return 1
    print("\nAll installer tests passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
