param([switch]$Launch, [switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $SkipBuild) {
    Push-Location $repository
    try {
        & cargo build --workspace --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'Preview build failed.' }
    } finally { Pop-Location }
}
$version = '0.1.0-' + (Get-Date -Format 'yyyyMMdd-HHmmss')
$destination = Join-Path $repository "dist\$version"
New-Item -ItemType Directory -Path $destination -Force | Out-Null
foreach ($name in @('Reversebolt.exe','reversebolt-mcp.exe')) {
    Copy-Item -LiteralPath (Join-Path $repository "target\release\$name") -Destination (Join-Path $destination $name)
}
Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination $destination
$manifest = Get-ChildItem -LiteralPath $destination -Filter '*.exe' | ForEach-Object {
    [pscustomobject]@{ name=$_.Name; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
}
$manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $destination 'manifest.json') -Encoding utf8
$version | Set-Content -LiteralPath (Join-Path $repository 'dist\current.txt') -Encoding utf8
$desktop = [Environment]::GetFolderPath('Desktop')
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path $desktop 'Reversebolt.lnk'))
$shortcut.TargetPath = (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe')
$launcher = Join-Path $repository 'scripts\launch-preview.ps1'
$shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$launcher`""
$shortcut.WorkingDirectory = $repository
$shortcut.Description = 'Reversebolt — discover, investigate, and edit'
$shortcut.IconLocation = Join-Path $destination 'Reversebolt.exe'
$shortcut.Save()
Write-Output "Published preview: $destination"
Write-Output "Desktop shortcut: $(Join-Path $desktop 'Reversebolt.lnk')"
if ($Launch) { & $launcher }
