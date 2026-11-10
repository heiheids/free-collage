param(
    [Parameter(Mandatory=$true)][int]$X,
    [Parameter(Mandatory=$true)][int]$Y,
    [int]$ProcessId = 0
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RealClickTool {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
    [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint x,uint y,uint d,IntPtr e);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
}
"@

[RealClickTool]::SetThreadDpiAwarenessContext([IntPtr](-4)) | Out-Null
if ($ProcessId -le 0) {
    $proc = Get-Process PuzzleStudio -ErrorAction Stop | Select-Object -First 1
} else {
    $proc = Get-Process -Id $ProcessId -ErrorAction Stop
}
$h = [IntPtr]$proc.MainWindowHandle
[RealClickTool]::ShowWindow($h, 9) | Out-Null
[RealClickTool]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 250
$r = New-Object RealClickTool+RECT
[RealClickTool]::GetWindowRect($h, [ref]$r) | Out-Null
[RealClickTool]::SetCursorPos($r.L + $X, $r.T + $Y) | Out-Null
Start-Sleep -Milliseconds 80
[RealClickTool]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero)
[RealClickTool]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 250
Write-Output ("CLICK {0},{1} at screen {2},{3}" -f $X, $Y, ($r.L + $X), ($r.T + $Y))
