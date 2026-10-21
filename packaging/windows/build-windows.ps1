# Builds a self-contained Windows (x64) .zip of PQ Studio: the app, the USGS
# PHREEQC engine and its databases. Run on Windows in PowerShell, from anywhere:
#
#     powershell -ExecutionPolicy Bypass -File packaging\windows\build-windows.ps1
#
# Needs: Visual Studio 2022 or its Build Tools with the "Desktop development
# with C++" workload (includes CMake), and Rust via rustup (https://rustup.rs,
# the default MSVC toolchain). If cmake is not on PATH, run this from a
# "Developer PowerShell for VS 2022".
#
# Writes dist\out\PQ-Studio-<version>-windows-x64.zip. Users unzip it and run
# "PQ Studio.exe" inside; nothing is installed.
#
# Optional: $env:PHREEQC_TARBALL = path to an already-downloaded source tarball.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-Native {
    # Native programs don't throw on failure; check their exit code.
    $exe, $rest = $args
    & $exe @rest
    if ($LASTEXITCODE -ne 0) { throw "$exe failed with exit code $LASTEXITCODE" }
}

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$cfg = @{}
foreach ($line in Get-Content (Join-Path $Root 'packaging\phreeqc.env')) {
    if ($line -match '^([A-Z0-9_]+)=(.+)$') { $cfg[$Matches[1]] = $Matches[2].Trim() }
}
foreach ($key in 'PHREEQC_VERSION', 'PHREEQC_TAG', 'PHREEQC_SHA256') {
    if (-not $cfg[$key]) { throw "$key missing from packaging\phreeqc.env" }
}
$PhreeqcVersion = $cfg['PHREEQC_VERSION']
$PhreeqcUrl = "https://github.com/phreeqc-dev/phreeqc3/releases/download/$($cfg['PHREEQC_TAG'])/phreeqc-$PhreeqcVersion.tar.gz"
$Version = (Select-String -Path (Join-Path $Root 'Cargo.toml') -Pattern '^version = "(.*)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value

$Work = Join-Path $Root 'packaging\windows\build'
$Out = Join-Path $Root 'dist\out'
# Windows' own bsdtar: Git's GNU tar on PATH reads "C:\..." as a remote host.
$Tar = Join-Path $env:SystemRoot 'System32\tar.exe'
foreach ($tool in 'cmake', 'cargo', 'rustup', $Tar) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "'$tool' not found (see the comments at the top of this script)"
    }
}
if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
New-Item -ItemType Directory -Force -Path $Work, $Out | Out-Null

Write-Host "==> PHREEQC engine $PhreeqcVersion"
$tarball = $env:PHREEQC_TARBALL
if (-not $tarball) {
    $tarball = Join-Path $Work "phreeqc-$PhreeqcVersion.tar.gz"
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $PhreeqcUrl -OutFile $tarball
}
$hash = (Get-FileHash -Algorithm SHA256 $tarball).Hash
if ($hash -ne $cfg['PHREEQC_SHA256']) { throw "checksum mismatch for $tarball ($hash)" }
Invoke-Native $Tar -xzf $tarball -C $Work
$src = Join-Path $Work "phreeqc-$PhreeqcVersion"

# Unmodified upstream source; build configuration only (see dist\NOTICE.md).
# The C runtime is linked statically so no Visual C++ Redistributable is needed.
$build = Join-Path $Work 'phreeqc-build'
Invoke-Native cmake -S $src -B $build -A x64 `
    -DPHRQC_USE_GMP=OFF `
    -DCMAKE_POLICY_DEFAULT_CMP0091=NEW `
    -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded
Invoke-Native cmake --build $build --config Release --target phreeqc
$engine = Join-Path $build 'Release\phreeqc.exe'
if (-not (Test-Path $engine)) { throw "phreeqc engine binary not produced" }

Write-Host '==> engine smoke test'
$smoke = Join-Path $Work 'smoke'
New-Item -ItemType Directory -Force -Path $smoke | Out-Null
$db = Join-Path $src 'database\phreeqc.dat'
Set-Content -Path (Join-Path $smoke 'in.pqi') -Encoding ascii -Value @(
    "DATABASE $db", 'SOLUTION 1', '    pH 7', '    Na 1', '    Cl 1', 'END')
Push-Location $smoke
try { & $engine in.pqi out.pqo *> $null } finally { Pop-Location }
if (-not (Select-String -Quiet -Path (Join-Path $smoke 'out.pqo') -Pattern 'End of simulation')) {
    throw 'engine smoke test failed'
}

Write-Host '==> app'
$target = 'x86_64-pc-windows-msvc'
Invoke-Native rustup target add $target
$env:RUSTFLAGS = '-C target-feature=+crt-static'
Invoke-Native cargo build --manifest-path (Join-Path $Root 'Cargo.toml') --release --target $target
Remove-Item Env:RUSTFLAGS

Write-Host '==> package'
$name = "PQ-Studio-$Version-windows-x64"
$stage = Join-Path $Work 'PQ Studio'
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'database'), (Join-Path $stage 'licenses') | Out-Null
Copy-Item (Join-Path $Root "target\$target\release\pq-studio.exe") (Join-Path $stage 'PQ Studio.exe')
Copy-Item $engine (Join-Path $stage 'phreeqc.exe')
Copy-Item (Join-Path $src 'database\*.dat') (Join-Path $stage 'database')
foreach ($f in 'LICENSE', 'NOTICE.md', 'THIRD_PARTY_LICENSES.md', 'FONT_LICENSE.txt') {
    Copy-Item (Join-Path $Root "dist\$f") (Join-Path $stage "licenses\$f")
}
Copy-Item (Join-Path $src 'doc\NOTICE') (Join-Path $stage 'licenses\PHREEQC_NOTICE.txt')
Copy-Item (Join-Path $Root 'dist\README.md') (Join-Path $stage 'README.md')

$zip = Join-Path $Out "$name.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $stage -DestinationPath $zip

Write-Host ''
Write-Host "Built: $zip"
