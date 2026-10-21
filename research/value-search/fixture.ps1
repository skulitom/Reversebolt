param([string]$ControlFile,[string]$OracleFile,[int]$Variant)
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class DistantNumberWindow : Form {
  [DllImport("kernel32.dll")] static extern IntPtr VirtualAlloc(IntPtr at, UIntPtr length, uint kind, uint protection);
  [DllImport("kernel32.dll")] static extern bool VirtualFree(IntPtr at, UIntPtr length, uint kind);
  readonly IntPtr arena,field; readonly int variant; readonly string control,oracle;
  readonly Timer timer=new Timer();string last="";
  double Read(){var b=new byte[variant==1?8:4];Marshal.Copy(field,b,0,b.Length);return (variant==1?BitConverter.ToDouble(b,0):BitConverter.ToSingle(b,0))*(variant==2?2:1);}
  void Set(double v){var b=variant==1?BitConverter.GetBytes(v):BitConverter.GetBytes((float)(v/(variant==2?2:1)));Marshal.Copy(b,0,field,b.Length);}
  public DistantNumberWindow(string c,string o,int kind) {
    control=c;oracle=o;variant=kind;
    Text="Reversebolt distant-value fixture";ClientSize=new Size(600,220);
    BackColor=Color.FromArgb(24,26,30);ForeColor=Color.White;Font=new Font("Segoe UI",36);
    arena=VirtualAlloc(IntPtr.Zero,(UIntPtr)(128*1024*1024),0x3000,4);if(arena==IntPtr.Zero)throw new Exception("allocation failed");
    field=IntPtr.Add(arena,64*1024*1024+new Random().Next(0,8*1024*1024-2)*8);Set(40);
    timer.Interval=50;timer.Tick+=(a,b)=>{
      if(File.Exists(control)){
        string next;try{next=File.ReadAllText(control).Trim();}catch(IOException){return;}
        if(next=="close"){Close();return;}
        if(next!=last && next.StartsWith("set:")){double v;if(double.TryParse(next.Substring(4),out v))Set(v);last=next;}
      }
      string json="{\"field\":"+field.ToInt64()+",\"arena\":"+arena.ToInt64()+",\"variant\":"+variant+",\"value\":"+Read().ToString(System.Globalization.CultureInfo.InvariantCulture)+"}";
      try{File.WriteAllText(oracle,json);}catch(IOException){}
      Invalidate();
    };timer.Start();
  }
  protected override void OnPaint(PaintEventArgs e){TextRenderer.DrawText(e.Graphics,"Quantity "+Read().ToString("0"),Font,new Point(30,65),ForeColor,BackColor);}
  protected override void OnFormClosed(FormClosedEventArgs e){timer.Stop();timer.Dispose();VirtualFree(arena,UIntPtr.Zero,0x8000);base.OnFormClosed(e);}
}
'@
[System.Windows.Forms.Application]::Run((New-Object DistantNumberWindow($ControlFile,$OracleFile,$Variant)))
