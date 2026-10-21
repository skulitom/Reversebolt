param([string]$Binary, [string]$Directory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$workers = @()
$form = New-Object Windows.Forms.Form
$form.Text = 'Reversebolt process-family fixture'
$form.Size = New-Object Drawing.Size(520, 300)
$form.StartPosition = 'Manual'
$form.Location = New-Object Drawing.Point(10, 10)
$label = New-Object Windows.Forms.Label
$label.Dock = 'Fill'
$label.Font = New-Object Drawing.Font('Arial', 30)
$label.TextAlign = 'MiddleCenter'
$form.Controls.Add($label)
$timer = New-Object Windows.Forms.Timer
$timer.Interval = 50
$script:last = -1
$watch = [Diagnostics.Stopwatch]::StartNew()
try {
    foreach ($seed in @(8433, 9401)) {
        $start = New-Object Diagnostics.ProcessStartInfo
        $start.FileName = $Binary
        $start.Arguments = "--fixture $seed"
        $start.UseShellExecute = $false
        $start.CreateNoWindow = $true
        $start.RedirectStandardInput = $true
        $start.RedirectStandardOutput = $true
        $process = [Diagnostics.Process]::Start($start)
        $hello = $process.StandardOutput.ReadLine() | ConvertFrom-Json
        $workers += [pscustomobject]@{ process=$process; hello=$hello }
    }
    @($workers | ForEach-Object { @{pid=$_.process.Id;base=$_.hello.base;length=$_.hello.length} }) |
        ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $Directory 'workers.json') -Encoding utf8
    $timer.Add_Tick({
        if ($watch.Elapsed.TotalSeconds -gt 600) { $form.Close(); return }
        $control = Join-Path $Directory 'control.json'
        if (-not (Test-Path -LiteralPath $control)) { return }
        try { $request = Get-Content -LiteralPath $control -Raw | ConvertFrom-Json } catch { return }
        if ($request.id -le $script:last) { return }
        $script:last = $request.id
        if ($request.op -eq 'close') { $form.Close(); return }
        $worker = $workers[$request.worker].process
        if ($request.op -eq 'kill') {
            $worker.StandardInput.WriteLine('quit'); $worker.StandardInput.Flush()
            if (-not $worker.WaitForExit(3000)) { throw 'Owned child did not quit.' }
            $reply = @{id=$request.id;exited=$worker.Id}
        } else {
            $worker.StandardInput.WriteLine('step'); $worker.StandardInput.Flush()
            $sample = $worker.StandardOutput.ReadLine() | ConvertFrom-Json
            $label.Text = "Quantity $($sample.observations[0])`r`nCharge $($sample.observations[1])"
            $form.Refresh()
            $reply = @{id=$request.id;observations=$sample.observations;pid=$worker.Id}
        }
        $reply | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Directory 'response.json') -Encoding utf8
    })
    $timer.Start()
    [Windows.Forms.Application]::Run($form)
} finally {
    $timer.Stop(); $timer.Dispose(); $form.Dispose()
    foreach ($worker in $workers) {
        if (-not $worker.process.HasExited) {
            $worker.process.StandardInput.WriteLine('quit'); $worker.process.StandardInput.Flush()
            if (-not $worker.process.WaitForExit(3000)) { $worker.process.Kill(); $worker.process.WaitForExit() }
        }
        $worker.process.Dispose()
    }
}
