param([Parameter(Mandatory=$true)][string]$ControlFile,[Parameter(Mandatory=$true)][string]$StateFile,[Parameter(Mandatory=$true)][string]$Title,[ValidateSet("state","history")][string]$Mode="state")
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class InputFixtureNative {
  [StructLayout(LayoutKind.Sequential)] public struct Mouse { public int x,y; public uint data,flags,time; public UIntPtr extra; }
  [StructLayout(LayoutKind.Sequential)] public struct Keyboard { public ushort key,scan; public uint flags,time; public UIntPtr extra; }
  [StructLayout(LayoutKind.Explicit)] public struct Union { [FieldOffset(0)] public Mouse mouse; [FieldOffset(0)] public Keyboard keyboard; }
  [StructLayout(LayoutKind.Sequential)] public struct Input { public uint type; public Union value; }
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] static extern uint SendInput(uint n, Input[] inputs, int size);
  public static void Key(ushort key, bool up) {
    var value=new Input { type=1, value=new Union { keyboard=new Keyboard { key=key, flags=up ? 2u : 0u } } };
    if(SendInput(1,new[]{value},Marshal.SizeOf(typeof(Input)))!=1) throw new Exception("Owned fixture input was not injected.");
  }
}
'@
$form=New-Object System.Windows.Forms.Form
$form.Text=$Title
$form.ClientSize=New-Object System.Drawing.Size(480,260)
$form.StartPosition='Manual'
$form.Location=New-Object System.Drawing.Point(650,120)
$form.BackColor=[System.Drawing.Color]::FromArgb(24,26,30)
$form.KeyPreview=$true
$label=New-Object System.Windows.Forms.Label
$label.Text='Quantity 500'
$label.ForeColor=[System.Drawing.Color]::White
$label.Font=New-Object System.Drawing.Font('Segoe UI',32)
$label.AutoSize=$true
$label.Location=New-Object System.Drawing.Point(40,90)
$form.Controls.Add($label)
$script:state=@{pid=$PID;window=0;ack=0;down=@{};up=@{};quantity=500;presses=0;flag=$false;error=$null}
$script:held=New-Object 'System.Collections.Generic.HashSet[int]'
$form.Add_KeyDown({param($sender,$event)
    $key=[string][int]$event.KeyCode
    $script:state.down[$key]=1+[int]$script:state.down[$key]
    $script:state.presses+=1
    if(($Mode -eq 'state' -and $script:state.quantity -gt 498) -or ($Mode -eq 'history' -and $script:state.presses -le 2)) {
        $script:state.quantity-=1
        $script:state.flag=-not $script:state.flag
    }
    $label.Text="Quantity $($script:state.quantity)"
})
$form.Add_KeyUp({param($sender,$event)
    $key=[string][int]$event.KeyCode
    $script:state.up[$key]=1+[int]$script:state.up[$key]
})
$timer=New-Object System.Windows.Forms.Timer
$timer.Interval=20
$timer.Add_Tick({
    $script:state.window=$form.Handle.ToInt64()
    try {
        if(Test-Path -LiteralPath $ControlFile) {
            $command=[IO.File]::ReadAllText($ControlFile)|ConvertFrom-Json
            if($command.seq -gt $script:state.ack) {
                switch($command.action) {
                    'focus' { $form.Activate(); [void][InputFixtureNative]::SetForegroundWindow($form.Handle) }
                    'set' { $script:state.quantity=[double]$command.value; $label.Text="Quantity $($script:state.quantity)" }
                    'down' {
                        if([InputFixtureNative]::GetForegroundWindow() -ne $form.Handle) {throw 'Refusing input outside this owned fixture.'}
                        [InputFixtureNative]::Key([uint16]$command.key,$false)
                        [void]$script:held.Add([int]$command.key)
                    }
                    'up' { if($script:held.Contains([int]$command.key)) { [InputFixtureNative]::Key([uint16]$command.key,$true); [void]$script:held.Remove([int]$command.key) } }
                    'close' { foreach($key in $script:held) {[InputFixtureNative]::Key([uint16]$key,$true)}; $script:held.Clear(); $form.Close() }
                    default {throw 'Unknown fixture command.'}
                }
                $script:state.ack=$command.seq
            }
        }
    } catch { $script:state.error=$_.Exception.Message }
    $script:state.foreground=([InputFixtureNative]::GetForegroundWindow() -eq $form.Handle)
    [IO.File]::WriteAllText($StateFile,($script:state|ConvertTo-Json -Depth 5))
})
$timer.Start()
[System.Windows.Forms.Application]::Run($form)
$timer.Dispose()
$form.Dispose()
