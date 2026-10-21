param([Parameter(Mandatory=$true)][string]$ControlFile)
# Owned native integration fixture. Discovery is not given its pointer or layout.
# A command changes memory once; the display continuously reads that memory, so
# an external edit remains visible until the next distinct command.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class TrialWindow : Form {
  [DllImport("kernel32.dll")] static extern IntPtr VirtualAlloc(IntPtr at, UIntPtr length, uint kind, uint protection);
  [DllImport("kernel32.dll")] static extern bool VirtualFree(IntPtr at, UIntPtr length, uint kind);
  readonly IntPtr region;
  readonly IntPtr field;
  readonly string control;
  readonly Timer timer = new Timer();
  string last = "";
  public TrialWindow(string path) {
    control=path;
    Text="Reversebolt trial fixture"; ClientSize=new Size(560,260);
    BackColor=Color.FromArgb(24,26,30); ForeColor=Color.White;
    Font=new Font("Segoe UI",36);
    region=VirtualAlloc(IntPtr.Zero,(UIntPtr)4096,0x3000,4);
    if(region==IntPtr.Zero) throw new Exception("allocation failed");
    field=IntPtr.Add(region,new Random().Next(8,496)*8);
    Marshal.Copy(BitConverter.GetBytes(40.0f),0,field,4);
    timer.Interval=50;
    timer.Tick+=(s,e)=>{
      if(File.Exists(control)) {
        string next;
        try {next=File.ReadAllText(control).Trim();}
        catch(IOException) {return;} // The automated driver may be replacing it.
        if(next=="close") {Close();return;}
        if(next!=last) {
          float value;
          if(float.TryParse(next,out value)) Marshal.Copy(BitConverter.GetBytes(value),0,field,4);
          last=next;
        }
      }
      Invalidate();
    };
    timer.Start();
  }
  protected override void OnPaint(PaintEventArgs e) {
    var bytes=new byte[4];Marshal.Copy(field,bytes,0,4);
    TextRenderer.DrawText(e.Graphics,"Quantity "+BitConverter.ToSingle(bytes,0).ToString("0"),Font,new Point(30,70),ForeColor,BackColor);
  }
  protected override void OnFormClosed(FormClosedEventArgs e) {
    timer.Stop();timer.Dispose();VirtualFree(region,UIntPtr.Zero,0x8000);base.OnFormClosed(e);
  }
}
'@
[System.Windows.Forms.Application]::Run((New-Object TrialWindow($ControlFile)))
