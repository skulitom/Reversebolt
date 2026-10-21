param([string]$ControlFile,[string]$OracleFile)
# Disposable moving-allocation fixture. The oracle is evaluation-only.
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;
public class MovingWindow : Form {
  [DllImport("kernel32.dll")] static extern IntPtr VirtualAlloc(IntPtr at, UIntPtr length, uint kind, uint protection);
  [DllImport("kernel32.dll")] static extern bool VirtualFree(IntPtr at, UIntPtr length, uint kind);
  readonly IntPtr rootPage,root; readonly int offset; readonly string control,oracle;
  readonly List<IntPtr> retained=new List<IntPtr>(); readonly Timer timer=new Timer();
  string last="";int generation=0;
  static IntPtr Allocate(){var p=VirtualAlloc(IntPtr.Zero,(UIntPtr)4096,0x3000,4);if(p==IntPtr.Zero)throw new Exception("allocation failed");return p;}
  IntPtr Field(){return IntPtr.Add(Marshal.ReadIntPtr(root),offset);}
  float Read(){var b=new byte[4];Marshal.Copy(Field(),b,0,4);return BitConverter.ToSingle(b,0);}
  void Set(float v){Marshal.Copy(BitConverter.GetBytes(v),0,Field(),4);}
  void Relocate(bool release){var previous=Marshal.ReadIntPtr(root);float v=Read();Marshal.WriteIntPtr(root,Allocate());Set(v);generation++;if(release)VirtualFree(previous,UIntPtr.Zero,0x8000);else retained.Add(previous);}
  public MovingWindow(string c,string o) {
    control=c;oracle=o;var random=new Random();
    Text="Reversebolt moving-value fixture";ClientSize=new Size(600,220);
    BackColor=Color.FromArgb(24,26,30);ForeColor=Color.White;Font=new Font("Segoe UI",36);
    rootPage=Allocate();root=IntPtr.Add(rootPage,random.Next(8,400)*8);offset=random.Next(8,400)*8;
    Marshal.WriteIntPtr(root,Allocate());Set(40f);
    timer.Interval=50;timer.Tick+=(a,b)=>{
      if(File.Exists(control)){
        string next;try{next=File.ReadAllText(control).Trim();}catch(IOException){return;}
        if(next=="close"){Close();return;}
        if(next!=last){if(next.StartsWith("move:"))Relocate(next=="move:free");else if(next.StartsWith("set:")){float v;if(float.TryParse(next.Substring(4),out v))Set(v);}last=next;}
      }
      string json="{\"field\":"+Field().ToInt64()+",\"root\":"+root.ToInt64()+",\"offset\":"+offset+",\"generation\":"+generation+",\"value\":"+Read().ToString(System.Globalization.CultureInfo.InvariantCulture)+"}";
      try{File.WriteAllText(oracle,json);}catch(IOException){}
      Invalidate();
    };timer.Start();
  }
  protected override void OnPaint(PaintEventArgs e){TextRenderer.DrawText(e.Graphics,"Quantity "+Read().ToString("0"),Font,new Point(30,65),ForeColor,BackColor);}
  protected override void OnFormClosed(FormClosedEventArgs e){timer.Stop();timer.Dispose();VirtualFree(Marshal.ReadIntPtr(root),UIntPtr.Zero,0x8000);foreach(var p in retained)VirtualFree(p,UIntPtr.Zero,0x8000);VirtualFree(rootPage,UIntPtr.Zero,0x8000);base.OnFormClosed(e);}
}
'@
[System.Windows.Forms.Application]::Run((New-Object MovingWindow($ControlFile,$OracleFile)))
