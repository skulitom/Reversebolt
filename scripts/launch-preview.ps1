param([switch]$CheckOnly)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
function Get-PreviewState {
    $current = Join-Path $repository 'dist\current.txt'
    if (-not (Test-Path -LiteralPath $current -PathType Leaf)) {
        return [pscustomobject]@{ status='not_published'; message='No desktop preview has been published yet.' }
    }
    $version = (Get-Content -LiteralPath $current -Raw).Trim()
    if ($version -cnotmatch '^\d+\.\d+\.\d+-\d{8}-\d{6}$') {
        return [pscustomobject]@{ status='invalid_version'; message='The desktop preview reference is invalid.' }
    }
    $directory = Join-Path $repository "dist\$version"
    $missing = @('Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll') | Where-Object {
        -not (Test-Path -LiteralPath (Join-Path $directory $_) -PathType Leaf)
    }
    if ($missing) {
        return [pscustomobject]@{ status='unavailable'; version=$version; missing=@($missing);
            message='Some preview files are unavailable. If Windows Security removed them, resolve that alert before trying again. Your saved session has not been changed.' }
    }
    [pscustomobject]@{ status='ready'; version=$version; executable=(Join-Path $directory 'Reversebolt.exe') }
}
# Allows automated availability checks without opening windows or launching any executable.
if ($CheckOnly) { Get-PreviewState | ConvertTo-Json; exit }
try {
    $endpointPath = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Reversebolt\endpoint.json'
    if (Test-Path -LiteralPath $endpointPath) {
        try {
            $endpoint = Get-Content -LiteralPath $endpointPath -Raw | ConvertFrom-Json
            $running = Get-Process -Id $endpoint.pid -ErrorAction Stop
            if ($running.ProcessName -eq 'Reversebolt') {
                # A second launch focuses the existing window; it never replaces an active session.
                $shell = New-Object -ComObject WScript.Shell
                if ($shell.AppActivate($running.Id)) { exit }
            }
        } catch { }
    }
    $preview = Get-PreviewState
    if ($preview.status -ne 'ready') { throw $preview.message }
    # This is the visible application the user explicitly asked to try.
    Start-Process -FilePath $preview.executable -WorkingDirectory $repository
} catch {
    $message = "Reversebolt could not open.`r`n`r`n" + $_.Exception.Message
    try {
        Add-Type -AssemblyName System.Windows.Forms
        [void][System.Windows.Forms.MessageBox]::Show($message, 'Reversebolt unavailable', 'OK', 'Information')
    } catch {
        Write-Warning $message
    }
    exit 1
}
