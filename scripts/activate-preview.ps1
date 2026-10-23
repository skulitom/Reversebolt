param(
    [Parameter(Mandatory)]
    [ValidatePattern('^\d+\.\d+\.\d+-\d{8}-\d{6}$')]
    [string]$Version
)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$destination = Join-Path $repository "dist\$Version"
$expected = @('Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll')
$manifest = @(Get-Content -LiteralPath (Join-Path $destination 'manifest.json') -Raw | ConvertFrom-Json)
if ($manifest.Count -ne $expected.Count -or @($manifest.name | Select-Object -Unique).Count -ne $expected.Count) {
    throw 'The staged preview manifest must identify exactly four distinct binaries.'
}
foreach ($entry in $manifest) {
    if ($entry.name -cnotin $expected -or $entry.sha256 -notmatch '^[a-fA-F0-9]{64}$') {
        throw 'The staged preview manifest contains an unexpected binary or hash.'
    }
    if ((Get-FileHash -LiteralPath (Join-Path $destination $entry.name) -Algorithm SHA256).Hash -ne $entry.sha256) {
        throw "Refusing to activate a mismatched preview binary: $($entry.name)"
    }
}
$desktop = [Environment]::GetFolderPath('Desktop')
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $desktop 'Reversebolt.lnk'))
$shortcut.TargetPath = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$launcher = Join-Path $repository 'scripts\launch-preview.ps1'
$shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$launcher`""
$shortcut.WorkingDirectory = $repository
$shortcut.Description = 'Reversebolt — discover, investigate, and edit'
$shortcut.IconLocation = Join-Path $destination 'Reversebolt.exe'
$shortcut.Save()
$Version | Set-Content -LiteralPath (Join-Path $repository 'dist\current.txt') -Encoding utf8
Write-Output "Published preview: $destination"
Write-Output "Desktop shortcut: $(Join-Path $desktop 'Reversebolt.lnk')"
