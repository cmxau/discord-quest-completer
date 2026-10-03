# Draws the app icon (a "Q" ring with a check mark on an indigo tile) as a 1024x1024 PNG.
#
#   pwsh scripts/make-icon.ps1                       # writes src-tauri/icons/app-icon-source.png
#   pnpm tauri icon src-tauri/icons/app-icon-source.png   # generates every size/format from it
param(
    [string]$Out = "src-tauri/icons/app-icon-source.png"
)

Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$g.Clear([System.Drawing.Color]::Transparent)

function New-RoundedRect([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    return $p
}

function New-Pen($color, [float]$width) {
    $pen = New-Object System.Drawing.Pen $color, $width
    $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
    return $pen
}

# --- Tile: diagonal indigo -> violet gradient ---
$tile = New-RoundedRect 32 32 960 960 224
$rect = New-Object System.Drawing.RectangleF 0, 0, $size, $size
$grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush $rect,
    ([System.Drawing.Color]::FromArgb(255, 79, 70, 229)),   # indigo-600
    ([System.Drawing.Color]::FromArgb(255, 124, 58, 237)),  # violet-600
    45
$g.FillPath($grad, $tile)

# Soft highlight along the top edge
$hi = New-Object System.Drawing.Drawing2D.LinearGradientBrush (New-Object System.Drawing.RectangleF 0, 0, $size, 520),
    ([System.Drawing.Color]::FromArgb(70, 255, 255, 255)),
    ([System.Drawing.Color]::FromArgb(0, 255, 255, 255)),
    90
$g.SetClip($tile)
$g.FillRectangle($hi, 0, 0, $size, 520)
$g.ResetClip()

# --- The "Q": a ring with a tail ---
$cx = 500; $cy = 490; $radius = 270; $ringWidth = 112
$ringRect = New-Object System.Drawing.RectangleF ($cx - $radius), ($cy - $radius), (2 * $radius), (2 * $radius)
# A short tail that crosses the ring (flat inner end, round outer end) so the mark reads as a "Q"
$tailFrom = New-Object System.Drawing.PointF 610, 600
$tailTo = New-Object System.Drawing.PointF 780, 770

# drop shadow first (offset, translucent), then the white shapes
$shadow = [System.Drawing.Color]::FromArgb(60, 20, 10, 80)
$shadowPen = New-Pen $shadow $ringWidth
$tailShadowPen = New-Pen $shadow $ringWidth
$tailShadowPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Flat
$g.TranslateTransform(0, 18)
$g.DrawEllipse($shadowPen, $ringRect)
$g.DrawLine($tailShadowPen, $tailFrom, $tailTo)
$g.ResetTransform()

$white = New-Pen ([System.Drawing.Color]::White) $ringWidth
$g.DrawEllipse($white, $ringRect)
$tailPen = New-Pen ([System.Drawing.Color]::White) $ringWidth
$tailPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Flat
$g.DrawLine($tailPen, $tailFrom, $tailTo)

# --- Check mark inside the ring (emerald) ---
$check = New-Pen ([System.Drawing.Color]::FromArgb(255, 52, 211, 153)) 96
$pts = @(
    (New-Object System.Drawing.PointF 385, 495),
    (New-Object System.Drawing.PointF 468, 580),
    (New-Object System.Drawing.PointF 628, 392)
)
$checkShadow = New-Pen ([System.Drawing.Color]::FromArgb(50, 10, 40, 40)) 96
$g.TranslateTransform(0, 12)
$g.DrawLines($checkShadow, [System.Drawing.PointF[]]$pts)
$g.ResetTransform()
$g.DrawLines($check, [System.Drawing.PointF[]]$pts)

$g.Dispose()
$dir = Split-Path -Parent $Out
if ($dir) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Host "Wrote $Out"
