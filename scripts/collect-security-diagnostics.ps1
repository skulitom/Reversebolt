param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$targetExe = Join-Path $repoRoot 'target\release\Reversebolt.exe'
$frozenPath = Join-Path $repoRoot 'test-results\comparison-planner-v1\frozen.json'
$frozen = Get-Content -LiteralPath $frozenPath -Raw | ConvertFrom-Json
$sampleHash = (Get-FileHash -LiteralPath $targetExe -Algorithm SHA256).Hash.ToLowerInvariant()
if ($sampleHash -ne $frozen.binaries.'Reversebolt.exe') {
    throw 'The available executable is no longer the detected build. Do not substitute another sample.'
}
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Use a new diagnostics directory.' }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputRoot | Out-Null

# Read existing events only. No scan, execution, restoration, exclusion or upload.
$events = @(Get-WinEvent -FilterHashtable @{
    LogName='Microsoft-Windows-Windows Defender/Operational'
    Id=1000,1001,1002,1116,1117
    StartTime=(Get-Date).AddDays(-1)
} -MaxEvents 256 -ErrorAction SilentlyContinue)
$parsed = @($events | ForEach-Object {
    $xmlEvent = [xml]$_.ToXml()
    $data = @{}
    foreach ($entry in $xmlEvent.Event.EventData.Data) { $data[[string]$entry.Name] = [string]$entry.InnerText }
    [pscustomobject]@{ id=$_.Id; time_utc=$_.TimeCreated.ToUniversalTime().ToString('o'); data=$data }
})
$related = @($parsed | Where-Object {
    $_.data['Path'] -like "*$repoRoot\*" -or $_.data['Scan Resources'] -like "*$targetExe*"
})
$scanIds = @($related | Where-Object id -EQ 1000 | ForEach-Object { $_.data['Scan ID'] })
$allowed = @('Product Version','Detection Time','Threat ID','Threat Name','Severity Name',
    'Category Name','Source Name','Action Name','Error Code','Execution Name','Path',
    'Process Name','Security intelligence Version','Engine Version','Scan ID',
    'Scan Resources','Scan Parameters','Scan Trigger')
$sanitized = @($parsed | Where-Object {
    $_ -in $related -or ($_.data['Scan ID'] -and $_.data['Scan ID'] -in $scanIds)
} | ForEach-Object {
    $filtered = [ordered]@{}
    foreach ($name in $allowed) {
        if (-not $_.data.ContainsKey($name)) { continue }
        $value = $_.data[$name]
        if ($name -eq 'Process Name') { $value = [IO.Path]::GetFileName($value) }
        $filtered[$name] = $value -ireplace [regex]::Escape($repoRoot), '<repository>'
    }
    [pscustomobject]@{ id=$_.id; time_utc=$_.time_utc; data=$filtered }
})
$status = Get-MpComputerStatus | Select-Object AMProductVersion,AMEngineVersion,
    AntivirusSignatureVersion,AntivirusSignatureLastUpdated,RealTimeProtectionEnabled,
    AntivirusEnabled,IsTamperProtected
$signature = Get-AuthenticodeSignature -LiteralPath $targetExe
$record = [ordered]@{
    collected_utc=(Get-Date).ToUniversalTime().ToString('o')
    sample=[ordered]@{relative_path='target/release/Reversebolt.exe';sha256=$sampleHash;
        bytes=(Get-Item -LiteralPath $targetExe).Length;signature_status=$signature.Status.ToString();
        matches_pre_detection_frozen_hash=$true}
    defender=$status
    events=$sanitized
    interpretation='An on-demand scan and a prior behavioral detection are different evidence. A completed scan does not clear or explain the behavioral alert.'
    collection_actions='Read file identity, signature, Defender status and repository-related events only. No protection change, executable launch, restoration or upload.'
}
$record | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $outputRoot 'diagnostics.json') -Encoding utf8
$provenance = [ordered]@{
    parent_commit=$frozen.parent
    frozen_manifest_sha256=(Get-FileHash -LiteralPath $frozenPath -Algorithm SHA256).Hash.ToLowerInvariant()
    binaries=$frozen.binaries
    source_hashes=$frozen.sources
    note='The detected build includes uncommitted source changes after parent_commit. The exact frozen source hashes identify it; current main has additional changes. Game storage manifests are deliberately excluded.'
}
$provenance | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputRoot 'provenance.json') -Encoding utf8
Write-Output "Diagnostics saved: $outputRoot"
