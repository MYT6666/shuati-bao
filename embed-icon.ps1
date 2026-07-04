# 嵌入应用图标到 exe 的 PE 资源
# 用途：Tauri 默认不会把 icon.ico 嵌入 exe 文件本身，导致 Windows 资源管理器显示默认 Tauri 图标
# 这个脚本在 tauri build 完成后调用，自动把图标嵌入

param(
    [string]$ExePath = "src-tauri\target\release\tauri-app.exe",
    [string]$IcoPath = "src-tauri\icons\icon.ico",
    [string]$RceditPath = "rcedit.exe"
)

$ErrorActionPreference = "Stop"

# 检查 rcedit 是否存在
if (-not (Test-Path $RceditPath)) {
    Write-Host "[*] 首次运行，下载 rcedit..." -ForegroundColor Yellow
    try {
        Invoke-WebRequest -Uri "https://github.com/electron/rcedit/releases/download/v1.1.1/rcedit-x64.exe" -OutFile $RceditPath -UseBasicParsing
    } catch {
        Write-Host "[!] rcedit 下载失败：$($_.Exception.Message)" -ForegroundColor Red
        Write-Host "[!] 请手动从 https://github.com/electron/rcedit/releases 下载 rcedit-x64.exe 并放到项目根目录" -ForegroundColor Red
        exit 1
    }
}

# 检查 exe
if (-not (Test-Path $ExePath)) {
    Write-Host "[!] 找不到 exe: $ExePath" -ForegroundColor Red
    Write-Host "[!] 请先运行: npx tauri build --no-bundle" -ForegroundColor Yellow
    exit 1
}

# 检查 ico
if (-not (Test-Path $IcoPath)) {
    Write-Host "[!] 找不到图标: $IcoPath" -ForegroundColor Red
    exit 1
}

# 嵌入图标
Write-Host "[*] 正在嵌入图标..." -ForegroundColor Cyan
& .\rcedit.exe $ExePath --set-icon $IcoPath
if ($LASTEXITCODE -ne 0) {
    Write-Host "[!] 图标嵌入失败" -ForegroundColor Red
    exit 1
}

Write-Host "[OK] 图标嵌入成功" -ForegroundColor Green
Write-Host "[*] 验证方法：右键 exe → 属性 → 详细信息，可看到我们的 logo 图标" -ForegroundColor Gray

# 刷新 Windows 图标缓存（避免资源管理器显示旧图标）
Write-Host "[*] 刷新 Windows 图标缓存..." -ForegroundColor Cyan
ie4uinit.exe -show 2>$null
# touch 文件更新 mtime
$touchTime = (Get-Date)
(Get-Item $ExePath).LastWriteTime = $touchTime
Write-Host "[OK] 完成" -ForegroundColor Green
