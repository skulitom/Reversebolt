param([string]$ControlFile,[string]$OracleFile)
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Reversebolt persistent blank fixture'
$form.ClientSize = New-Object System.Drawing.Size(600,220)
$form.BackColor = [System.Drawing.Color]::FromArgb(24,26,30)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 50
$timer.Add_Tick({
    if ((Test-Path -LiteralPath $ControlFile) -and (Get-Content -LiteralPath $ControlFile -Raw) -eq 'close') { $form.Close() }
    $form.Invalidate()
})
$form.Add_Shown({
    [IO.File]::WriteAllText($OracleFile, '{"ready":true}')
    $timer.Start()
})
try { [System.Windows.Forms.Application]::Run($form) } finally { $timer.Stop(); $timer.Dispose(); $form.Dispose() }
