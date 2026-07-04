# 刷题宝

> 智能题库练习桌面应用 · 导入即刷 · AI 自动解析 · 永久免费

![logo](public/logo-256.png)

一款基于 Tauri + Vue 3 的桌面刷题应用，支持 Word/PDF/TXT/Markdown 导入，AI 智能解析题目，多种学习模式，让刷题更高效。

## ✨ 核心功能

- 📚 **题库管理** — 导入 / 收藏 / 错题本 / 学习记录
- 🤖 **AI 智能解析** — 支持多模型（智谱 / DeepSeek / Agnes）
- 📥 **多格式导入** — Word(.docx) / PDF / TXT / Markdown
- 🎯 **三种模式** — 顺序 / 随机 / 错题重做
- 📊 **学习统计** — 365 天热力图 / 正确率 / 连续天数
- ⌨️ **快捷键** — ←→ / A-D / Enter / F / R / Esc
- 🔄 **自动更新** — 一键升级到最新版本

## 🖼️ 界面预览

应用主界面、练习页、统计热力图、答题卡 logo 等。

## 🔧 系统要求

- Windows 10 / 11 (x64)
- 约 50 MB 磁盘空间
- 无需其他依赖（已内置 WebView2）

## 📥 下载安装

前往 [Releases 页面](https://github.com/MYT6666/shuati-bao/releases) 下载最新版：

- **安装包**（推荐）：`刷题宝_0.1.0_x64-setup.exe`
- **绿色版**：单文件 `tauri-app.exe`

## 🚀 快速开始

1. 下载安装包并运行
2. 点击 **导入** → 选择 .docx / .pdf / .txt / .md 文件
3. AI 自动解析题库（首次使用需在设置页配置 AI API Key）
4. 选择题库 → 开始刷题
5. 题目下方有导航条，点击切换题号
6. 答错的题自动加入错题本

### 配置 AI（可选）

设置页 → AI 解析：
- 智谱 AI：base_url = `https://open.bigmodel.cn/api/paas/v4`
- DeepSeek：base_url = `https://api.deepseek.com`
- Agnes：base_url = `https://api.agnes.cn/v1`

## ⌨️ 快捷键

| 键 | 作用 |
|---|---|
| ← / → | 上一题 / 下一题 |
| A / B / C / D | 选择选项 |
| Enter | 确认 / 下一题 |
| F | 收藏 / 取消收藏 |
| R | 重置 |
| ? | 显示所有快捷键 |

## 🛠️ 技术栈

- **前端**：Vue 3 + TypeScript + Vite
- **后端**：Rust + Tauri 2
- **数据库**：SQLite (本地存储)
- **UI**：原生 CSS 变量主题（亮/暗）

## 📝 开源协议

MIT License

## 💬 反馈

欢迎提 Issue 或 PR！
