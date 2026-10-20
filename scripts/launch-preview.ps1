$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$version = (Get-Content -LiteralPath (Join-Path $repository 'dist\current.txt') -Raw).Trim()
$executable = Join-Path $repository "dist\$version\Reversebolt.exe"
if (-not (Test-Path -LiteralPath $executable)) { throw 'The preview has not been built.' }
$endpointPath = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Reversebolt\endpoint.json'
if (Test-Path -LiteralPath $endpointPath) {
    try {
        $endpoint = Get-Content -LiteralPath $endpointPath -Raw | ConvertFrom-Json
        $running = Get-Process -Id $endpoint.pid -ErrorAction Stop
        if ($running.ProcessName -eq 'Reversebolt') {
            # A second launch focuses the existing window; it never replaces an active session.
            $shell = New-Object -ComObject WScript.Shell
            [void]$shell.AppActivate($running.Id)
            exit
        }
    } catch { }
}
# This is the visible application the user explicitly asked to try.
Start-Process -FilePath $executable -WorkingDirectory $repository
