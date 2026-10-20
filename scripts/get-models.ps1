param([string]$CacheDirectory)
$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$modelDirectory = Join-Path $repository 'models'
$manifest = Get-Content -LiteralPath (Join-Path $modelDirectory 'manifest.json') -Raw | ConvertFrom-Json
foreach ($model in $manifest.models) {
    if ($model.name -notmatch '^text-(detection|recognition)\.onnx$') { throw 'Unexpected model filename.' }
    $destination = Join-Path $modelDirectory $model.name
    if ((Test-Path -LiteralPath $destination) -and
        ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash -eq $model.sha256)) {
        Write-Output "Verified $($model.name)"
        continue
    }
    $temporary = "$destination.download"
    $cached = if ($CacheDirectory) { Join-Path $CacheDirectory $model.name } else { $null }
    if ($cached -and (Test-Path -LiteralPath $cached)) {
        Copy-Item -LiteralPath $cached -Destination $temporary
    } else {
        Invoke-WebRequest -Uri $model.url -OutFile $temporary
    }
    if ((Get-Item -LiteralPath $temporary).Length -ne $model.bytes -or
        (Get-FileHash -LiteralPath $temporary -Algorithm SHA256).Hash -ne $model.sha256) {
        throw "Pinned model verification failed: $($model.name). The downloaded file was retained for inspection."
    }
    Copy-Item -LiteralPath $temporary -Destination $destination
    Remove-Item -LiteralPath $temporary
    Write-Output "Installed and verified $($model.name)"
}
