param([switch]$Launch, [switch]$SkipBuild, [switch]$StageOnly, [string]$TargetDirectory = 'target')
$ErrorActionPreference = 'Stop'
if ($Launch -and $StageOnly) { throw 'A staged preview must be checked before it is activated or launched.' }
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$buildDirectory = if ([IO.Path]::IsPathRooted($TargetDirectory)) {
    [IO.Path]::GetFullPath($TargetDirectory)
} else {
    [IO.Path]::GetFullPath((Join-Path $repository $TargetDirectory))
}
if (-not $SkipBuild) {
    Push-Location $repository
    try {
        & cargo build --workspace --release --locked --target-dir $buildDirectory
        if ($LASTEXITCODE -ne 0) { throw 'Preview build failed.' }
    } finally { Pop-Location }
}
$version = '0.1.0-' + (Get-Date -Format 'yyyyMMdd-HHmmss')
$destination = Join-Path $repository "dist\$version"
New-Item -ItemType Directory -Path $destination -Force | Out-Null
foreach ($name in @('Reversebolt.exe','reversebolt-mcp.exe','reversebolt-runtime-loader.exe','reversebolt_runtime.dll')) {
    Copy-Item -LiteralPath (Join-Path $buildDirectory "release\$name") -Destination (Join-Path $destination $name)
}
Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination $destination
$publishedLicenses = Join-Path $destination 'licenses'
New-Item -ItemType Directory -Path $publishedLicenses | Out-Null
Get-ChildItem -LiteralPath (Join-Path $repository 'licenses') -File | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $publishedLicenses
}
$modelDirectory = Join-Path $repository 'models'
$modelManifest = Get-Content -LiteralPath (Join-Path $modelDirectory 'manifest.json') -Raw | ConvertFrom-Json
$modelsAvailable = @($modelManifest.models | Where-Object {
    Test-Path -LiteralPath (Join-Path $modelDirectory $_.name)
}).Count -eq $modelManifest.models.Count
if ($modelsAvailable) {
    foreach ($model in $modelManifest.models) {
        if ((Get-FileHash -LiteralPath (Join-Path $modelDirectory $model.name) -Algorithm SHA256).Hash -ne $model.sha256) {
            throw "Refusing to publish a mismatched OCR model: $($model.name)"
        }
    }
    $publishedModels = Join-Path $destination 'models'
    New-Item -ItemType Directory -Path $publishedModels | Out-Null
    foreach ($name in @('manifest.json','NOTICE.md') + @($modelManifest.models.name)) {
        $publishedModelPath = Join-Path $publishedModels $name
        New-Item -ItemType Directory -Path (Split-Path -Parent $publishedModelPath) -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $modelDirectory $name) -Destination $publishedModelPath
    }
} else {
    Write-Warning 'Publishing without optional neural models; only Windows OCR will be available.'
}
$manifest = Get-ChildItem -LiteralPath $destination -File | Where-Object { $_.Extension -in @('.exe','.dll') } | ForEach-Object {
    [pscustomobject]@{ name=$_.Name; sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
}
$manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $destination 'manifest.json') -Encoding utf8
if ($StageOnly) {
    Write-Output "Staged preview: $destination"
    return
}
& (Join-Path $PSScriptRoot 'activate-preview.ps1') -Version $version
if ($Launch) { & (Join-Path $PSScriptRoot 'launch-preview.ps1') }
