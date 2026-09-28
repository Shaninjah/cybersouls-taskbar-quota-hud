# Loads only function definitions; never installs, uninstalls, or edits the registry.
$ErrorActionPreference = 'Stop'
$Repository = 'Shaninjah/cybersouls-taskbar-quota-hud'
$checks = 0

function Import-SafetyFunctions {
    param([string]$File)
    $errors = $null
    $tokens = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($File, [ref]$tokens, [ref]$errors)
    if ($errors.Count) { throw 'PowerShell script syntax is invalid.' }
    foreach ($statement in $ast.EndBlock.Statements) {
        if ($statement -is [System.Management.Automation.Language.FunctionDefinitionAst]) {
            . ([scriptblock]::Create($statement.Extent.Text))
        }
    }
    # Return the definitions so the caller can load them in its own scope.
    return $ast.EndBlock.Statements | Where-Object {
        $_ -is [System.Management.Automation.Language.FunctionDefinitionAst]
    } | ForEach-Object { $_.Extent.Text }
}

function Assert-Rejected {
    param([scriptblock]$Action)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw 'Expected unsafe input to be rejected.' }
    $script:checks++
}

foreach ($definition in Import-SafetyFunctions (Join-Path $PSScriptRoot 'install.ps1')) {
    . ([scriptblock]::Create($definition))
}
$name = 'cybersouls-taskbar-quota-hud.exe'
$validUrl = "https://github.com/$Repository/releases/download/v1.9.2/$name"
Assert-ReleaseAssetUrl -Url $validUrl -Name $name
$checks++
foreach ($url in @(
    $validUrl.Replace('https:', 'http:'),
    $validUrl.Replace('github.com/', 'github.com.evil.invalid/'),
    $validUrl.Replace($Repository, 'upstream-ray/codex-usage-monitor'),
    $validUrl.Replace('v1.9.2', '..'),
    $validUrl.Replace('v1.9.2', 'v1.9.2/extra'),
    "${validUrl}?query=1",
    "${validUrl}#fragment",
    $validUrl.Replace($name, 'other.exe')
)) {
    Assert-Rejected { Assert-ReleaseAssetUrl -Url $url -Name $name }
}

$hash = 'a' * 64
if ((Read-ReleaseChecksum "$hash  $name`n") -ne $hash) { throw 'Valid checksum was rejected.' }
$checks++
foreach ($text in @($hash, "$hash  other.exe", "prefix $hash  $name", "$hash  $name`n$hash  $name", "$('a' * 63)  $name")) {
    Assert-Rejected { Read-ReleaseChecksum $text }
}

foreach ($definition in Import-SafetyFunctions (Join-Path $PSScriptRoot 'uninstall.ps1')) {
    . ([scriptblock]::Create($definition))
}
$temporaryParent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$testRoot = Join-Path $temporaryParent ('cybersouls-safety-test-' + [Guid]::NewGuid().ToString('N'))
$junction = Join-Path $testRoot 'junction'
Assert-NoReparsePoint -Path $testRoot
New-Item -ItemType Directory -Path (Join-Path $testRoot 'outside') -Force | Out-Null
try {
    Set-Content -LiteralPath (Join-Path $testRoot 'outside\keep.txt') -Value 'synthetic test data'
    Assert-SafeRemovalTree -Path $testRoot
    $checks++
    New-Item -ItemType Junction -Path $junction -Target (Join-Path $testRoot 'outside') | Out-Null
    Assert-Rejected { Assert-NoReparsePoint -Path (Join-Path $junction 'keep.txt') }
    Assert-Rejected { Assert-SafeRemovalTree -Path $testRoot }
    if (-not (Test-Path -LiteralPath (Join-Path $testRoot 'outside\keep.txt'))) { throw 'Guard modified unrelated data.' }
    $checks++
}
finally {
    # Remove the junction itself first, then verify the bounded tree before cleanup.
    if (Test-Path -LiteralPath $junction) {
        if (-not ((Get-Item -LiteralPath $junction -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Expected a junction at the bounded test path.'
        }
        # Directory.Delete without recursion removes the link itself on PS 5.1 too.
        [IO.Directory]::Delete($junction)
    }
    $resolvedRoot = [IO.Path]::GetFullPath($testRoot)
    if (-not $resolvedRoot.StartsWith($temporaryParent + '\cybersouls-safety-test-', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing unexpected test cleanup path.'
    }
    Assert-SafeRemovalTree -Path $resolvedRoot
    Remove-Item -LiteralPath $resolvedRoot -Recurse -Force
}
Write-Output "Installer safety: $checks checks passed."
