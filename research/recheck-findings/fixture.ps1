param([string]$ControlFile,[string]$OracleFile)
# Owned, disposable two-location fixture. The oracle is evaluation-only.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class ProbeWindow : Form {
  [DllImport("kernel32.dll")] static extern IntPtr VirtualAlloc(IntPtr at, UIntPtr length, uint kind, uint protection);
  [DllImport("kernel32.dll")] static extern bool VirtualFree(IntPtr at, UIntPtr length, uint kind);
  readonly IntPtr region; readonly IntPtr[] fields; readonly string control,oracle;
  readonly Timer timer=new Timer(); string last="";
  public ProbeWindow(string c,string o) {
    control=c; oracle=o;
    Text="Reversebolt location probe fixture";ClientSize=new Size(600,300);
    BackColor=Color.FromArgb(24,26,30);ForeColor=Color.White;Font=new Font("Segoe UI",36);
    region=VirtualAlloc(IntPtr.Zero,(UIntPtr)4096,0x3000,4);
    if(region==IntPtr.Zero)throw new Exception("allocation failed");
    var random=new Random();
    fields=new[]{IntPtr.Add(region,random.Next(8,220)*8),IntPtr.Add(region,random.Next(260,496)*8)};
    foreach(var f in fields)Marshal.Copy(BitConverter.GetBytes(40f),0,f,4);
    timer.Interval=50;timer.Tick+=(a,b)=>{
      if(File.Exists(control)) {
        string next;try{next=File.ReadAllText(control).Trim();}catch(IOException){return;}
        if(next=="close"){Close();return;}
        if(next!=last){var parts=next.Split(',');if(parts.Length==2){for(int i=0;i<2;i++){float v;if(float.TryParse(parts[i],out v))Marshal.Copy(BitConverter.GetBytes(v),0,fields[i],4);}}last=next;}
      }
      var json="{\"fields\":["+fields[0].ToInt64()+","+fields[1].ToInt64()+"],\"display_fields\":[0,1],\"values\":["+Read(0).ToString(System.Globalization.CultureInfo.InvariantCulture)+","+Read(1).ToString(System.Globalization.CultureInfo.InvariantCulture)+"]}";
      try{File.WriteAllText(oracle,json);}catch(IOException){}
      Invalidate();
    };timer.Start();
  }
  float Read(int i){var b=new byte[4];Marshal.Copy(fields[i],b,0,4);return BitConverter.ToSingle(b,0);}
  protected override void OnPaint(PaintEventArgs e){TextRenderer.DrawText(e.Graphics,"Quantity "+Read(0).ToString("0"),Font,new Point(30,45),ForeColor,BackColor);TextRenderer.DrawText(e.Graphics,"Reserve "+Read(1).ToString("0"),Font,new Point(30,150),ForeColor,BackColor);}
  protected override void OnFormClosed(FormClosedEventArgs e){timer.Stop();timer.Dispose();VirtualFree(region,UIntPtr.Zero,0x8000);base.OnFormClosed(e);}
}
'@
[System.Windows.Forms.Application]::Run((New-Object ProbeWindow($ControlFile,$OracleFile)))
