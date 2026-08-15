param()

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$SidecarDir = Join-Path $Root "sidecars/hy-mt"
$DistDir = Join-Path $SidecarDir "dist/hy-mt-sidecar"
$BundleDir = Join-Path $SidecarDir "bundle"

if ($env:OS -ne "Windows_NT") {
  throw "Build the HY-MT sidecar on the same operating system as the desktop release."
}
if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
  throw "The HY-MT sidecar build requires uv: https://docs.astral.sh/uv/"
}

Push-Location $SidecarDir
try {
  uv sync --frozen --group build
  uv run --frozen --group build pyinstaller --noconfirm --clean hy-mt-poc.spec
} finally {
  Pop-Location
}

$Executable = Join-Path $DistDir "hy-mt-sidecar.exe"
if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
  throw "PyInstaller completed without creating $Executable."
}

New-Item -ItemType Directory -Force -Path $BundleDir | Out-Null
$ResolvedSidecar = (Resolve-Path -LiteralPath $SidecarDir).Path
$ResolvedBundle = (Resolve-Path -LiteralPath $BundleDir).Path
if (-not $ResolvedBundle.StartsWith($ResolvedSidecar, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to replace a bundle directory outside the HY-MT sidecar workspace."
}
Get-ChildItem -LiteralPath $BundleDir -Force |
  Where-Object Name -ne ".gitkeep" |
  Remove-Item -Recurse -Force
Copy-Item -Path (Join-Path $DistDir "*") -Destination $BundleDir -Recurse -Force

$FileCount = (Get-ChildItem -LiteralPath $BundleDir -Recurse -File).Count
$BundleBytes = (Get-ChildItem -LiteralPath $BundleDir -Recurse -File | Measure-Object Length -Sum).Sum
Write-Host "HY-MT sidecar ready: $FileCount files, $([math]::Round($BundleBytes / 1MB, 1)) MiB"
