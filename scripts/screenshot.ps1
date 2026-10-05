<#
.SYNOPSIS
Saves a PNG of a running CS2 Helper window's content (no title bar).

.DESCRIPTION
Uses Win32 PrintWindow with PW_RENDERFULLCONTENT, which captures WebView2
content even when the window is covered by other windows. The size is in
physical pixels, so for an HD image raise the Windows display scale
(Settings > Display > Scale, e.g. 200%) before capturing. The app must be
running (e.g. `pnpm tauri dev`) and the window not minimized.

.EXAMPLE
pnpm screenshot:windows
Captures the main window into site/public/screenshot.png.

.EXAMPLE
pnpm screenshot:windows -Title "CS2 Helper Overlay" -Out overlay.png
Captures the overlay window. Its transparent areas come out black, and an
idle overlay is all black: click Preview first, then run this within 5 s.
#>
param(
	[string] $Title = "CS2 Helper",
	[string] $Out = "site/public/screenshot.png"
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class ScreenshotWin {
	[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
	[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
	[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
	[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
	[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
}
"@

# Per-monitor DPI aware (v2): sizes are physical pixels at the display's scale.
[void][ScreenshotWin]::SetProcessDpiAwarenessContext([IntPtr](-4))

# [NullString]: a plain $null would be passed as "" and match no window class.
$h = [ScreenshotWin]::FindWindow([NullString]::Value, $Title)
if ($h -eq [IntPtr]::Zero) { throw "Window '$Title' not found. Is the app running?" }

$r = New-Object ScreenshotWin+RECT
[void][ScreenshotWin]::GetClientRect($h, [ref]$r)
$w = $r.R - $r.L
$hgt = $r.B - $r.T
if ($w -le 0 -or $hgt -le 0) { throw "Window '$Title' has no visible content (minimized?)" }

$bmp = New-Object System.Drawing.Bitmap $w, $hgt, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
# PW_CLIENTONLY (1) | PW_RENDERFULLCONTENT (2)
$ok = [ScreenshotWin]::PrintWindow($h, $hdc, 3)
$g.ReleaseHdc($hdc)
$g.Dispose()
if (-not $ok) {
	$bmp.Dispose()
	throw "PrintWindow failed for '$Title'"
}

# Relative to the current folder, or absolute; the file need not exist yet.
$path = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Out)
$bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"Saved $path (${w}x${hgt})"
