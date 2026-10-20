param([Parameter(Mandatory=$true)][string]$ControlFile)
# Owned test window. The observer receives captured pixels, never these shapes.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Reversebolt appearance fixture'
$form.ClientSize = New-Object System.Drawing.Size(480,320)
$form.BackColor = [System.Drawing.Color]::FromArgb(24,26,30)
$form.Tag = 'round'
$form.Add_Paint({
    param($sender,$event)
    $brush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(118,170,255))
    if ($sender.Tag -eq 'wide') { $event.Graphics.FillRectangle($brush,120,130,240,60) }
    else { $event.Graphics.FillEllipse($brush,190,110,100,100) }
    $brush.Dispose()
})
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 50
$timer.Add_Tick({
    if (Test-Path -LiteralPath $ControlFile) {
        $command = [IO.File]::ReadAllText($ControlFile).Trim()
        if ($command -eq 'close') { $form.Close() }
        elseif ($command -in @('round','wide') -and $form.Tag -ne $command) {
            $form.Tag = $command
            $form.Invalidate()
        }
    }
})
$timer.Start()
[System.Windows.Forms.Application]::Run($form)
$timer.Dispose()
$form.Dispose()
