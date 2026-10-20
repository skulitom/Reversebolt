param([Parameter(Mandatory=$true)][string]$ControlFile)
# Test-only WinForms window. Its layout and state are never supplied to the engine.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Reversebolt observation fixture'
$form.ClientSize = New-Object System.Drawing.Size(560,260)
$form.BackColor = [System.Drawing.Color]::FromArgb(24,26,30)
$label = New-Object System.Windows.Forms.Label
$label.Text = 'Quantity 40'
$label.ForeColor = [System.Drawing.Color]::White
$label.Font = New-Object System.Drawing.Font('Segoe UI',36)
$label.Location = New-Object System.Drawing.Point(30,60)
$label.Size = New-Object System.Drawing.Size(500,100)
$form.Controls.Add($label)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 50
$timer.Add_Tick({
    if (Test-Path -LiteralPath $ControlFile) {
        $command = [IO.File]::ReadAllText($ControlFile).Trim()
        if ($command -eq 'close') { $form.Close() }
        elseif ($command -match '^\d{1,3}$') { $label.Text = "Quantity $command" }
    }
})
$timer.Start()
[System.Windows.Forms.Application]::Run($form)
$timer.Dispose()
$form.Dispose()
