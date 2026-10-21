param([Parameter(Mandatory=$true)][string]$Image, [Parameter(Mandatory=$true)][string]$ControlFile)
# Owned native window for replay plumbing; no game, labels or memory layout are supplied to inference.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$picture = [System.Drawing.Image]::FromFile($Image)
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Reversebolt image replay fixture'
$form.ClientSize = New-Object System.Drawing.Size(960,540)
$form.StartPosition = 'Manual'
$form.Location = New-Object System.Drawing.Point(30,30)
$box = New-Object System.Windows.Forms.PictureBox
$box.Dock = 'Fill'
$box.SizeMode = 'Zoom'
$box.Image = $picture
$form.Controls.Add($box)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 100
$timer.Add_Tick({
    if ((Test-Path -LiteralPath $ControlFile) -and [IO.File]::ReadAllText($ControlFile).Trim() -eq 'close') { $form.Close() }
})
$timer.Start()
[System.Windows.Forms.Application]::Run($form)
$timer.Dispose()
$form.Dispose()
$picture.Dispose()
