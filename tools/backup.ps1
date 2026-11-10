param(
    [string]$Label = "edit"
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$backupDir = Join-Path $root "拼图坊工程备份"
New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

$safeLabel = ($Label -replace '[^\w\u4e00-\u9fff-]', '_')
$stamp = Get-Date -Format "yyyyMMdd_HHmmss_fff"
$zipPath = Join-Path $backupDir "拼图坊_${stamp}_${safeLabel}.zip"

$candidates = @(
    "src",
    "languages",
    "assets",
    "tests",
    "tools",
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "README.md",
    ".cargo"
)

$items = @()
foreach ($item in $candidates) {
    $full = Join-Path $root $item
    if (Test-Path $full) { $items += $full }
}

if ($items.Count -eq 0) {
    throw "没有可备份的工程文件：$root"
}

Compress-Archive -Path $items -DestinationPath $zipPath -CompressionLevel Optimal -Force

$log = Join-Path $backupDir "backup-log.txt"
$line = "{0}  {1}  {2}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $safeLabel, (Split-Path -Leaf $zipPath)
Add-Content -Path $log -Value $line -Encoding utf8

$size = (Get-Item $zipPath).Length
Write-Output ("BACKUP_OK {0} ({1:N0} bytes)" -f $zipPath, $size)
