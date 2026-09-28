# Keep compiler source locations from embedding the builder's personal paths.
$ErrorActionPreference = 'Stop'
$repositoryPath = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\', '/')
$flags = @()
if ($env:CARGO_ENCODED_RUSTFLAGS) {
    $flags += $env:CARGO_ENCODED_RUSTFLAGS.Split([char]31)
}
foreach ($mapping in @(
    @{ Path = $env:USERPROFILE; Replacement = '/user' },
    @{ Path = $repositoryPath; Replacement = '/workspace' }
)) {
    if (-not [string]::IsNullOrWhiteSpace($mapping.Path)) {
        $prefix = $mapping.Path.TrimEnd('\', '/')
        $flags += "--remap-path-prefix=$prefix=$($mapping.Replacement)"
        $flags += "--remap-path-prefix=$($prefix.Replace('\', '/'))=$($mapping.Replacement)"
    }
}
$env:CARGO_ENCODED_RUSTFLAGS = $flags -join [char]31
Push-Location -LiteralPath $repositoryPath
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw "Release build failed: $LASTEXITCODE" }
}
finally {
    Pop-Location
}
