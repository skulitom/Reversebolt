param([string]$ControlFile,[string]$OracleFile,[int]$SourceSlot=0)
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
  readonly int source; readonly Timer timer=new Timer(); string last="";
  public ProbeWindow(string c,string o,int s) {
    control=c; oracle=o; source=s;
    Text="Reversebolt location probe fixture";ClientSize=new Size(560,260);
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
        if(next!=last){float v;if(float.TryParse(next,out v))foreach(var f in fields)Marshal.Copy(BitConverter.GetBytes(v),0,f,4);last=next;}
      }
      var json="{\"fields\":["+fields[0].ToInt64()+","+fields[1].ToInt64()+"],\"source\":"+source+",\"values\":["+Read(0).ToString(System.Globalization.CultureInfo.InvariantCulture)+","+Read(1).ToString(System.Globalization.CultureInfo.InvariantCulture)+"]}";
      try{File.WriteAllText(oracle,json);}catch(IOException){}
      Invalidate();
    };timer.Start();
  }
  float Read(int i){var b=new byte[4];Marshal.Copy(fields[i],b,0,4);return BitConverter.ToSingle(b,0);}
  protected override void OnPaint(PaintEventArgs e){TextRenderer.DrawText(e.Graphics,"Quantity "+Read(source).ToString("0"),Font,new Point(30,70),ForeColor,BackColor);}
  protected override void OnFormClosed(FormClosedEventArgs e){timer.Stop();timer.Dispose();VirtualFree(region,UIntPtr.Zero,0x8000);base.OnFormClosed(e);}
}
'@
[System.Windows.Forms.Application]::Run((New-Object ProbeWindow($ControlFile,$OracleFile,$SourceSlot)))
