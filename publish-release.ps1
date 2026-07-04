# GitHub Release 一键发布脚本
# 用法：.\publish-release.ps1 -Version 0.1.0
# 前置：gh CLI 已登录（gh auth login）

param(
    [Parameter(Mandatory = $true)]
    [string]$Version = "0.1.0",

    [string]$Title = "ShuatiBao v0.1.0 - First Release"
)

$ErrorActionPreference = 'Stop'

# Check gh CLI
$ghCmd = Get-Command gh -ErrorAction SilentlyContinue
$useCurl = $false

if (-not $ghCmd) {
    # fallback: check curl
    $curlCmd = Get-Command curl -ErrorAction SilentlyContinue
    if (-not $curlCmd) {
        [Console]::WriteLine("[!] gh CLI / curl not installed. Download gh: https://cli.github.com/")
        exit 1
    }
    # require GITHUB_TOKEN
    if (-not $env:GITHUB_TOKEN) {
        [Console]::WriteLine("[!] gh not found, and GITHUB_TOKEN env var not set.")
        [Console]::WriteLine("    Set: `$env:GITHUB_TOKEN = 'ghp_xxx'")
        exit 1
    }
    $useCurl = $true
    [Console]::WriteLine("[OK] Using curl + GITHUB_TOKEN")
} else {
    # gh CLI requires login
    & gh auth status 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        [Console]::WriteLine("[!] Please login first: gh auth login")
        exit 1
    }
    [Console]::WriteLine("[OK] gh CLI authenticated")
}

# Check files
$setup = "src-tauri\target\release\bundle\nsis\刷题宝_${Version}_x64-setup.exe"
$sig = $setup + ".sig"
$portable = "src-tauri\target\release\tauri-app.exe"

if (-not (Test-Path $setup)) {
    [Console]::WriteLine("[!] Setup not found: $setup")
    exit 1
}
if (-not (Test-Path $sig)) {
    [Console]::WriteLine("[!] Signature missing: $sig")
    exit 1
}

[Console]::WriteLine("[OK] Setup exists")
[Console]::WriteLine("[OK] Signature exists")

# Collect upload files
$uploadArgs = @($setup, $sig)
if (Test-Path $portable) {
    $uploadArgs = $uploadArgs + @($portable)
    [Console]::WriteLine("[OK] Portable exists")
}

# 计算 SHA256
[Console]::WriteLine("")
[Console]::WriteLine("[*] Computing SHA256...")
$hashLines = New-Object System.Collections.Generic.List[string]
foreach ($f in $uploadArgs) {
    $h = (Get-FileHash $f -Algorithm SHA256).Hash
    $size = (Get-Item $f).Length
    $sizeKB = [math]::Round($size / 1KB, 1)
    $name = Split-Path $f -Leaf
    $line = "$h  $name  (${sizeKB} KB)"
    $hashLines.Add($line)
    [Console]::WriteLine("  $line")
}

# Generate release notes (using here-string)
$Notes = @"
## ShuatiBao v${Version} First Release

A smart desktop quiz app based on Tauri + Vue 3. Import Word/PDF/TXT/Markdown files, AI auto-parses questions, supports multiple study modes.

### Features

- **Question Bank Management** - Import / Favorites / Wrong Questions / Study Records
- **AI Auto-Parse** - Multi-model support (Zhipu / DeepSeek / Agnes)
- **Multi-Format Import** - Word (.docx) / PDF / TXT / Markdown
- **Three Study Modes** - Sequential / Random / Wrong-Only
- **Study Statistics** - 365-day heatmap / Accuracy / Streak
- **Keyboard Shortcuts** - A-D / Prev-Next / Enter / F / R
- **Auto-Update** - Built-in Tauri updater, push updates to all users

### Downloads

- **Installer (Recommended)**: ShuatiBao_${Version}_x64-setup.exe
- **Portable**: tauri-app.exe

### System Requirements

- Windows 10 / 11 (x64)
- ~50 MB disk space
- WebView2 built-in (no other dependencies)

### SHA256 Checksums

```
$(($hashLines -join "`n"))
```

### Quick Start

1. Download and run the installer
2. Click **Import** -> select .docx / .pdf / .txt / .md file
3. AI auto-parses the question bank
4. Select a bank and start practicing
5. Click navigation dots below question to switch

### AI Configuration (Optional)

Settings -> AI Parser:
- Zhipu AI: base_url = `https://open.bigmodel.cn/api/paas/v4`
- DeepSeek: base_url = `https://api.deepseek.com`
- Agnes: base_url = `https://api.agnes.cn/v1`

### License

MIT
"@

$notesFile = Join-Path $env:TEMP ("release-notes-" + $Version + ".md")
$Notes | Out-File -FilePath $notesFile -Encoding utf8

# Create Release
[Console]::WriteLine("")
[Console]::WriteLine("[*] Creating GitHub Release v$Version ...")

if ($useCurl) {
    # curl + GITHUB_TOKEN 模式
    # 1) 创建 release
    $createBody = @{
        tag_name = "v$Version"
        name = $Title
        body = $Notes
        target_commitish = "main"
        draft = $false
        prerelease = $false
    } | ConvertTo-Json -Depth 10

    $createBody | Out-File -FilePath "$notesFile.create.json" -Encoding utf8

    & curl.exe -s -X POST `
        -H "Authorization: token $env:GITHUB_TOKEN" `
        -H "Accept: application/vnd.github+json" `
        -H "Content-Type: application/json" `
        --data-binary "@$notesFile.create.json" `
        "https://api.github.com/repos/MYT6666/shuati-bao/releases" -o "$notesFile.resp.json"

    $respJson = Get-Content "$notesFile.resp.json" -Raw
    $releaseObj = $respJson | ConvertFrom-Json
    $uploadUrl = $releaseObj.upload_url
    if (-not $uploadUrl) {
        [Console]::WriteLine("[!] Create release failed: $respJson")
        exit 1
    }
    $uploadUrl = $uploadUrl -replace '\{.*\}$', ''

    # 2) 上传每个文件
    foreach ($f in $uploadArgs) {
        $name = Split-Path $f -Leaf
        [Console]::WriteLine("  Uploading $name ...")
        & curl.exe -s -X POST `
            -H "Authorization: token $env:GITHUB_TOKEN" `
            -H "Content-Type: application/octet-stream" `
            --data-binary "@$f" `
            "$uploadUrl?name=$([uri]::EscapeDataString($name))" -o "$notesFile.upload.json"
    }
} else {
    & gh release create "v$Version" @uploadArgs --repo MYT6666/shuati-bao --title $Title --notes-file $notesFile --target main
}

if ($LASTEXITCODE -eq 0) {
    [Console]::WriteLine("")
    [Console]::WriteLine("[OK] Release created successfully!")
    [Console]::WriteLine("    https://github.com/MYT6666/shuati-bao/releases/tag/v$Version")
} else {
    [Console]::WriteLine("")
    [Console]::WriteLine("[!] Release creation failed")
    exit 1
}
