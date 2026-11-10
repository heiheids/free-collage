param(
    [string]$OutDir = (Join-Path (Split-Path -Parent $PSScriptRoot) "assets")
)

Add-Type -AssemblyName System.Drawing

function New-RoundedPath([System.Drawing.RectangleF]$r, [float]$radius) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $radius * 2
    $p.AddArc($r.X, $r.Y, $d, $d, 180, 90)
    $p.AddArc($r.Right - $d, $r.Y, $d, $d, 270, 90)
    $p.AddArc($r.Right - $d, $r.Bottom - $d, $d, $d, 0, 90)
    $p.AddArc($r.X, $r.Bottom - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    return $p
}

function New-IconBitmap([int]$size) {
    $ss = 4
    $big = $size * $ss
    $bmp = New-Object System.Drawing.Bitmap($big, $big, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)
    $u = [float]$big

    $bgRect = New-Object System.Drawing.RectangleF(($u * 0.06), ($u * 0.06), ($u * 0.88), ($u * 0.88))
    $bg = New-RoundedPath $bgRect ($u * 0.22)
    $b = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 15, 118, 110))
    $g.FillPath($b, $bg); $b.Dispose(); $bg.Dispose()

    $c1Rect = New-Object System.Drawing.RectangleF(($u * 0.20), ($u * 0.22), ($u * 0.34), ($u * 0.28))
    $c1 = New-RoundedPath $c1Rect ($u * 0.06)
    $b = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $g.FillPath($b, $c1); $b.Dispose(); $c1.Dispose()

    $c2Rect = New-Object System.Drawing.RectangleF(($u * 0.48), ($u * 0.33), ($u * 0.32), ($u * 0.24))
    $c2 = New-RoundedPath $c2Rect ($u * 0.06)
    $b = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 253, 230, 138))
    $g.FillPath($b, $c2); $b.Dispose(); $c2.Dispose()

    $c3Rect = New-Object System.Drawing.RectangleF(($u * 0.28), ($u * 0.56), ($u * 0.44), ($u * 0.22))
    $c3 = New-RoundedPath $c3Rect ($u * 0.06)
    $b = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $g.FillPath($b, $c3); $b.Dispose(); $c3.Dispose()

    $b = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 251, 113, 133))
    $r = $u * 0.085
    $g.FillEllipse($b, ($u * 0.75 - $r), ($u * 0.72 - $r), ($r * 2), ($r * 2))
    $b.Dispose(); $g.Dispose()

    $small = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g2 = [System.Drawing.Graphics]::FromImage($small)
    $g2.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g2.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g2.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g2.DrawImage($bmp, (New-Object System.Drawing.Rectangle(0, 0, $size, $size)))
    $g2.Dispose(); $bmp.Dispose()
    return $small
}

function Get-PngBytes([System.Drawing.Bitmap]$bmp) {
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bytes = $ms.ToArray()
    $ms.Dispose()
    return $bytes
}

if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force -Path $OutDir | Out-Null }
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$images = @()
foreach ($s in $sizes) {
    $bmp = New-IconBitmap $s
    $images += ,(Get-PngBytes $bmp)
    if ($s -eq 256) { $bmp.Save((Join-Path $OutDir "icon-preview.png"), [System.Drawing.Imaging.ImageFormat]::Png) }
    $bmp.Dispose()
}

$icoPath = Join-Path $OutDir "app.ico"
$fs = [System.IO.File]::Create($icoPath)
$w = New-Object System.IO.BinaryWriter($fs)
$w.Write([uint16]0)
$w.Write([uint16]1)
$w.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $s = $sizes[$i]
    $w.Write([byte]$(if ($s -ge 256) { 0 } else { $s }))
    $w.Write([byte]$(if ($s -ge 256) { 0 } else { $s }))
    $w.Write([byte]0)
    $w.Write([byte]0)
    $w.Write([uint16]1)
    $w.Write([uint16]32)
    $w.Write([uint32]$images[$i].Length)
    $w.Write([uint32]$offset)
    $offset += $images[$i].Length
}
foreach ($data in $images) { $w.Write([byte[]]$data) }
$w.Flush(); $w.Dispose(); $fs.Dispose()
Write-Output ("ICON_OK " + $icoPath)
