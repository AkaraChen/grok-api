# Download the latest grok-api release for this machine (Windows PowerShell / pwsh).
# Usage: irm https://raw.githubusercontent.com/AkaraChen/grok-api/main/install.ps1 | iex
$ErrorActionPreference = 'Stop'
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
} catch {
    # Older runtimes may already negotiate TLS 1.2.
}

$Repo = if ($env:GROK_API_REPO) { $env:GROK_API_REPO } else { 'AkaraChen/grok-api' }
$GitHubApi = if ($env:GROK_API_GITHUB_API) { $env:GROK_API_GITHUB_API.TrimEnd('/') } else { 'https://api.github.com' }
$BinName = 'grok-api'
$ReleasesPage = "https://github.com/$Repo/releases/latest"

function Get-DefaultInstallDir {
    if ($env:GROK_API_INSTALL_DIR) { return $env:GROK_API_INSTALL_DIR }
    if ($env:LOCALAPPDATA) { return (Join-Path $env:LOCALAPPDATA 'grok-api') }
    return (Join-Path (Join-Path $HOME '.local') 'bin')
}

function Write-RateLimitHelp {
    $message = @"
error: GitHub API rate limit exceeded.

This installer lists the latest release via:
  $GitHubApi/repos/$Repo/releases/latest

Unauthenticated requests are tightly limited. You can:

  1. `$env:GITHUB_TOKEN = '...'`          # or GH_TOKEN
  2. install GitHub CLI and run: gh auth login
     (this script uses that OAuth token automatically when gh is logged in)
  3. download the archive for your machine from:
     $ReleasesPage
"@
    Write-Error $message
}

function Write-NoReleaseHelp {
    $message = @"
error: no GitHub release found for $Repo.

Publish binaries by pushing a version tag (for example v0.1.0), or install from source:

  cargo install --path .

Manual downloads (once a release exists):
  $ReleasesPage
"@
    Write-Error $message
}

function Get-InstallerToken {
    if ($env:GITHUB_TOKEN) { return $env:GITHUB_TOKEN }
    if ($env:GH_TOKEN) { return $env:GH_TOKEN }
    if (-not $env:GROK_API_SKIP_GH -and (Get-Command gh -ErrorAction SilentlyContinue)) {
        $token = & gh auth token 2>$null
        if ($LASTEXITCODE -eq 0 -and $token) { return ([string]$token).Trim() }
    }
    return $null
}

function Get-GitHubHeaders {
    $headers = @{
        Accept                   = 'application/vnd.github+json'
        'User-Agent'             = 'grok-api-installer'
        'X-GitHub-Api-Version'   = '2022-11-28'
    }
    $token = Get-InstallerToken
    if ($token) { $headers['Authorization'] = "Bearer $token" }
    return $headers
}

function Get-ReleaseTarget {
    if ($env:GROK_API_TARGET) { return $env:GROK_API_TARGET }

    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    $onWindows = ($env:OS -eq 'Windows_NT') -or ($PSVersionTable.PSEdition -eq 'Desktop')
    if ($onWindows) {
        if ($arch -eq 'Arm64') { return 'aarch64-pc-windows-msvc' }
        return 'x86_64-pc-windows-msvc'
    }

    $unix = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::OSX)
    if ($unix) {
        if ($arch -eq 'Arm64') { return 'aarch64-apple-darwin' }
        return 'x86_64-apple-darwin'
    }

    if ($arch -eq 'Arm64') { return 'aarch64-unknown-linux-gnu' }
    return 'x86_64-unknown-linux-gnu'
}

function Get-ArchiveName([string]$Target) {
    if ($Target -match 'windows') { return "$BinName-$Target.zip" }
    return "$BinName-$Target.tar.gz"
}

function Get-HttpStatus($ErrorRecord) {
    $response = $ErrorRecord.Exception.Response
    if (-not $response) { return 0 }
    try { return [int]$response.StatusCode } catch { return 0 }
}

function Read-HttpBody($ErrorRecord) {
    $response = $ErrorRecord.Exception.Response
    if (-not $response) { return '' }
    try {
        $stream = $response.GetResponseStream()
        if (-not $stream) { return '' }
        $reader = New-Object System.IO.StreamReader($stream)
        return $reader.ReadToEnd()
    } catch {
        return ''
    }
}

$InstallDir = Get-DefaultInstallDir
$Target = Get-ReleaseTarget
$Archive = Get-ArchiveName $Target
$Headers = Get-GitHubHeaders
if ($env:GROK_API_VERSION) {
    $ApiUrl = "$GitHubApi/repos/$Repo/releases/tags/$($env:GROK_API_VERSION)"
} else {
    $ApiUrl = "$GitHubApi/repos/$Repo/releases/latest"
}

try {
    $Release = Invoke-RestMethod -Uri $ApiUrl -Headers $Headers
} catch {
    $code = Get-HttpStatus $_
    $body = Read-HttpBody $_
    if ($code -eq 404) { Write-NoReleaseHelp; throw }
    if ($code -eq 429 -or $code -eq 403 -or ($body -match 'rate limit')) {
        Write-RateLimitHelp
        throw
    }
    Write-Error "GitHub API returned HTTP $code. Set GITHUB_TOKEN / run gh auth login, or download from $ReleasesPage"
    throw
}

if ($Release.html_url) { $ReleasesPage = $Release.html_url }

$Asset = $Release.assets | Where-Object { $_.name -eq $Archive } | Select-Object -First 1
if (-not $Asset) {
    Write-Error "latest release $($Release.tag_name) has no asset $Archive. Download from $ReleasesPage"
    throw 'missing release asset'
}

$Sums = $Release.assets | Where-Object { $_.name -in @('SHA256SUMS', 'sha256sums.txt') } | Select-Object -First 1

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("grok-api-install-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Work | Out-Null
try {
    $AssetPath = Join-Path $Work $Archive
    Invoke-WebRequest -Uri $Asset.browser_download_url -Headers $Headers -OutFile $AssetPath -UseBasicParsing

    if ($Sums) {
        $SumsPath = Join-Path $Work 'SHA256SUMS'
        try {
            Invoke-WebRequest -Uri $Sums.browser_download_url -Headers $Headers -OutFile $SumsPath -UseBasicParsing
            $expected = $null
            foreach ($line in Get-Content $SumsPath) {
                $parts = $line.Trim() -split '\s+'
                if ($parts.Count -ge 2 -and ($parts[-1] -eq $Archive -or $parts[-1] -eq "*$Archive")) {
                    $expected = $parts[0]
                    break
                }
            }
            $actual = (Get-FileHash -Algorithm SHA256 -Path $AssetPath).Hash.ToLowerInvariant()
            if ($expected -and $expected.ToLowerInvariant() -ne $actual) {
                throw "checksum mismatch for $Archive"
            }
        } catch {
            if ("$_" -match 'checksum mismatch') { throw }
            Write-Warning "could not verify SHA256SUMS: $_"
        }
    }

    $Extract = Join-Path $Work 'extract'
    New-Item -ItemType Directory -Path $Extract | Out-Null
    if ($Archive.EndsWith('.zip')) {
        Expand-Archive -Path $AssetPath -DestinationPath $Extract -Force
    } else {
        tar -xzf $AssetPath -C $Extract
    }

    $Src = Get-ChildItem -Path $Extract -Recurse -File |
        Where-Object { $_.Name -eq $BinName -or $_.Name -eq "$BinName.exe" } |
        Select-Object -First 1
    if (-not $Src) { throw "archive $Archive did not contain $BinName" }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $Dest = Join-Path $InstallDir $Src.Name
    Copy-Item -Path $Src.FullName -Destination $Dest -Force

    Write-Host "Installed $BinName $($Release.tag_name) to $Dest"

    if ($env:OS -eq 'Windows_NT') {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $userPath) { $userPath = '' }
        $parts = $userPath -split ';' | Where-Object { $_ }
        if ($parts -notcontains $InstallDir) {
            [Environment]::SetEnvironmentVariable('Path', (($parts + $InstallDir) -join ';'), 'User')
            $env:Path = "$InstallDir;$env:Path"
            Write-Host "Added $InstallDir to your user PATH. Open a new terminal if grok-api is not found."
        }
    } elseif ($env:PATH -notlike "*${InstallDir}*") {
        Write-Host "Add this directory to your PATH:`n  export PATH=`"$InstallDir`:`$PATH`""
    }
} finally {
    Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}
