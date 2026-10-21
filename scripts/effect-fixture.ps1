param(
    [Parameter(Mandatory=$true)][string]$ControlFile,
    [Parameter(Mandatory=$true)][ValidateSet('coupled','drift','hidden')][string]$Mode,
    [Parameter(Mandatory=$true)][string]$AuditFile
)
# Test-only truth stays in this external process and the post-close audit file.
# The discovery engine sees no field location, effect mode or control file.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class EffectWindow : Form {
  [DllImport("kernel32.dll")] static extern IntPtr VirtualAlloc(IntPtr at, UIntPtr length, uint kind, uint protection);
  [DllImport("kernel32.dll")] static extern bool VirtualFree(IntPtr at, UIntPtr length, uint kind);
  readonly IntPtr region;
  readonly IntPtr field;
  readonly string control, mode, audit;
  readonly Timer timer = new Timer();
  string last = "";
  float previous = 40;
  int resource = 25, hidden = 0, writes = 0;
  readonly DateTime started = DateTime.UtcNow;
  public EffectWindow(string path, string effectMode, string auditPath) {
    control=path;mode=effectMode;audit=auditPath;
    Text="Reversebolt trial fixture";ClientSize=new Size(560,320);
    BackColor=Color.FromArgb(24,26,30);ForeColor=Color.White;Font=new Font("Segoe UI",32);
    region=VirtualAlloc(IntPtr.Zero,(UIntPtr)4096,0x3000,4);
    if(region==IntPtr.Zero) throw new Exception("allocation failed");
    field=IntPtr.Add(region,new Random().Next(8,496)*8);
    Marshal.Copy(BitConverter.GetBytes(previous),0,field,4);
    timer.Interval=50;
    timer.Tick+=(s,e)=>{
      if(File.Exists(control)) {
        string next;
        try {next=File.ReadAllText(control).Trim();} catch(IOException) {return;}
        if(next=="close") {Close();return;}
        if(next!=last) {
          float value;
          if(float.TryParse(next,out value)) {
            Marshal.Copy(BitConverter.GetBytes(value),0,field,4);previous=value;
          }
          last=next;
        }
      }
      float current=Read();
      if(current!=previous) {
        writes++;
        if(mode=="coupled") resource-=3;
        if(mode=="hidden") hidden+=1;
        previous=current;
      }
      if(mode=="drift") resource=25+(int)(DateTime.UtcNow-started).TotalMilliseconds/100;
      Invalidate();
    };
    timer.Start();
  }
  float Read() {var bytes=new byte[4];Marshal.Copy(field,bytes,0,4);return BitConverter.ToSingle(bytes,0);}
  protected override void OnPaint(PaintEventArgs e) {
    TextRenderer.DrawText(e.Graphics,"Quantity "+Read().ToString("0"),Font,new Point(28,28),ForeColor,BackColor);
    TextRenderer.DrawText(e.Graphics,"Resource "+resource,Font,new Point(28,130),ForeColor,BackColor);
    // Nonnumeric appearance is available in the images, not scored as a number.
    e.Graphics.FillRectangle(mode=="coupled" && writes>0 ? Brushes.Orange : Brushes.CornflowerBlue,28,238,180,40);
  }
  protected override void OnFormClosed(FormClosedEventArgs e) {
    timer.Stop();timer.Dispose();
    File.WriteAllText(audit,"{\"mode\":\""+mode+"\",\"writes\":"+writes+",\"resource\":"+resource+",\"hidden\":"+hidden+",\"quantity\":"+Read()+"}");
    VirtualFree(region,UIntPtr.Zero,0x8000);base.OnFormClosed(e);
  }
}
'@
[System.Windows.Forms.Application]::Run((New-Object EffectWindow($ControlFile,$Mode,$AuditFile)))
