# 刷题宝 发布流程（自动更新）

## 🔐 密钥管理

密钥已生成（minisign 格式，**无密码**）：
- 私钥：`src-tauri/keys/updater.key`（**永远不要提交到 Git，已加入 .gitignore**）
- 公钥：`src-tauri/keys/updater.pub`（已嵌入 tauri.conf.json）

> ⚠️ **重要**：私钥丢失 = 以后所有用户都收不到更新提示。务必备份到密码管理器或离线 U 盘。

## 📋 发版流程

### 1. 改版本号

3 个地方必须同步：
- `src-tauri/Cargo.toml` → `version = "0.1.1"`
- `src-tauri/tauri.conf.json` → `"version": "0.1.1"`
- `package.json` → `"version": "0.1.1"`

### 2. 更新更新日志

在 `src/views/SettingsView.vue` 的 `<div class="log-entry">` 下追加新版本条目。

### 3. 构建 + 签名

```powershell
# 关闭运行中的 tauri-app
Stop-Process -Name tauri-app -Force -ErrorAction SilentlyContinue

# 1) 构建
cd D:\桌面\刷题宝
npx tauri build --bundles nsis

# 2) 签名 setup.exe（生成 .sig 文件，Tauri updater 必需）
minisign -S -s src-tauri\keys\updater.key `
  -m "src-tauri\target\release\bundle\nsis\刷题宝_0.1.1_x64-setup.exe" `
  -c "shuati-bao updater"

# minisign 生成 .minisig 文件，重命名为 .sig
Rename-Item "src-tauri\target\release\bundle\nsis\刷题宝_0.1.1_x64-setup.exe.minisig" `
  "src-tauri\target\release\bundle\nsis\刷题宝_0.1.1_x64-setup.exe.sig"

# 3) 嵌入图标到 tauri-app.exe（仅绿色版，NSIS 不需要）
.\embed-icon.ps1
```

构建成功后会在 `src-tauri/target/release/bundle/nsis/` 下生成两个文件：
- `刷题宝_0.1.1_x64-setup.exe`（安装包）
- `刷题宝_0.1.1_x64-setup.exe.sig`（minisign 签名文件，**必须上传**）

### 4. 一键发布

```powershell
# 安装 gh CLI 后登录
winget install GitHub.cli
gh auth login

# 一键发布（自动计算 SHA256 + 上传 3 个文件 + 创建 Release）
.\publish-release.ps1 -Version 0.1.1
```

### 5. 验证更新

让一个旧版本用户（或者自己用旧版）打开应用，启动 3 秒后会弹窗"发现新版本 v0.1.1"。

## ❗ 常见错误

| 错误 | 原因 | 解决 |
|---|---|---|
| 用户收到"未签名更新" | 上传时漏了 .sig 文件 | 重新上传含 .sig |
| 用户点击更新没反应 | endpoint 配错 | 检查 tauri.conf.json 的 endpoints URL |
| 编译错误"pubkey 格式不对" | 公钥不是 minisign 格式 | 用 `minisign -G -W` 重新生成 |
| 检查更新报错 403 | API rate limit | GitHub API 限制 60 次/小时 |

## 🔑 重新生成密钥（如丢失/泄露）

```powershell
# 生成新无密码 minisign 密钥
minisign -G -W -p src-tauri\keys\updater.pub -s src-tauri\keys\updater.key -c "shuati-bao updater"

# 提取新公钥
Get-Content src-tauri\keys\updater.pub

# 把 "untrusted comment: ..." + base64 那两行用 base64 编码后填到 tauri.conf.json 的 pubkey 字段
[Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes((Get-Content src-tauri\keys\updater.pub -Raw)))
```

## 📝 注意事项

1. **endpoint URL**：第一次配置时把 `tauri.conf.json` 的 `MYT6666/shuati-bao` 改成你真实的 GitHub 仓库路径
2. **.gitignore**：确保 `src-tauri/keys/updater.key` 在 .gitignore 中（密钥不能公开）
3. **首次发版**：手动下载安装即可，第二次起就支持自动更新
4. **签名格式**：Tauri 2 updater 接受 minisign 的 .sig 文件（本质是 .minisig 重命名）
