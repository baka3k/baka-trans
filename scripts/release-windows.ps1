param(
  [ValidateSet("check", "build")]
  [string]$Action = "check"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Require-Command([string]$Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    throw "Required command '$Name' was not found in PATH."
  }
}

Require-Command node
Require-Command npm
Require-Command cargo

if ($env:OS -ne "Windows_NT") {
  throw "The Windows installer must be built on Windows."
}

npm ci
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml

if ($Action -eq "build") {
  & (Join-Path $PSScriptRoot "build-vieneu-sidecar.ps1")
  & (Join-Path $PSScriptRoot "build-hy-mt-sidecar.ps1")
  npm run tauri -- build --bundles nsis
  $Bundle = Join-Path $Root "src-tauri/target/release/bundle/nsis"
  if (-not (Test-Path $Bundle)) {
    throw "NSIS build completed without creating $Bundle."
  }
  Get-ChildItem $Bundle -Filter "*.exe" | ForEach-Object {
    $Hash = Get-FileHash -Algorithm SHA256 $_.FullName
    "$($Hash.Hash)  $($_.Name)" | Set-Content -Encoding ascii "$($_.FullName).sha256"
  }
}
