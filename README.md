# 刷题宝 (ShuatiBao)

一款面向个人备考场景的**本地题库刷题工具**：把 Word 题库（.docx）一键导入，自动识别题目结构、答案、章节，支持顺序/随机/错题重练/模拟考试 4 种练习模式，配有收藏、计时、搜索、AI 解析等小工具。

**所有数据保存在你自己的电脑里**——不上传任何题库内容，AI 解析也只用你配置的 Key 直接调模型。

---

## ✨ 功能

| | |
|---|---|
| 📥 **题库导入** | 拖入 .docx → 自动识别题号、选项、答案、解析、章节；支持本地正则引擎（秒级）和 AI 引擎（兼容智谱/DeepSeek/Agnes/OpenAI） |
| 🎯 **4 种练习模式** | 顺序练习 / 随机练习 / 错题重练 / 模拟考试（60min 倒计时 + 自动交卷计分） |
| ⏱ **单题计时** | 每题自动计时，统计答题用时 |
| 🔖 **收藏 / 错题本** | 收藏喜欢的题，错题本可重练或标记"已掌握" |
| 🔍 **题目搜索** | 练习页按题干关键词搜索 |
| 🤖 **AI 单题解析** | 没解析的题可调用 AI 生成解析 |
| 💾 **数据本地化** | SQLite 存题库 + 错题 + 收藏 + 设置；一键备份数据库 |
| 🎨 **主题/字号** | 亮/暗/跟随系统三主题 + 小/中/大三档字号 |
| ⌨️ **快捷键** | `1-4` 选项、`Enter` 确认、`← →` 翻页、`F` 收藏 |
| 💝 **打赏作者** | 内置微信/支付宝收款码 |

## 📦 下载

前往 [Releases 页面](https://github.com/你的用户名/shuati-bao/releases) 下载最新的 `刷题宝_x.x.x_x64-setup.exe`（Windows 安装包，4.4MB）。

- 仅支持 Windows 10/11（64 位）
- 系统需自带 **WebView2**（Win11 默认有，Win10 1803+ 也自带）

## 🚀 开发

### 前置依赖

- Node.js ≥ 18
- Rust 工具链（含 `cargo`）
- Microsoft Visual Studio Build Tools（含 C++ 桌面开发 + Windows SDK）
- WebView2 Runtime（Win11 自带）

### 本地开发

```bash
npm install
npm run tauri dev
```

### 构建安装包

```bash
npm run tauri build -- --bundles nsis
```

构建产物：
- 单文件 exe：`src-tauri/target/release/tauri-app.exe`（17MB）
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/刷题宝_x.x.x_x64-setup.exe`（4.4MB）

## 🤖 AI 配置

1. 打开"设置 → AI 识别引擎"
2. 填入 API Key（智谱 GLM 送免费额度，DeepSeek 送 500 万 token）
3. 点击下方任一推荐模型按钮，一键填入 Base URL + 模型名
4. 点击"测试连接"验证

**推荐模型**：
- `glm-4-flash`（智谱，免费，速度快）
- `deepseek-chat`（DeepSeek，500万 token 免费）
- `agnes-2.0-flash`（Agnes，免费高并发）
- `gpt-4o-mini`（OpenAI，付费但便宜）

> ⚠ 不要用 `glm-4v`/`gpt-4o` 等视觉模型，纯文本任务慢 3-5 倍。

## 📂 数据存储位置

- 题库/错题/收藏：`%APPDATA%\com.shuati-bao.app\shuati.db`
- 数据库备份：同上目录下的 `backups/` 文件夹
- 调试日志：exe 同目录下的 `shuati-debug.log`

## 🛠 技术栈

- **前端**：Vue 3 + TypeScript + Pinia + Vue Router + Vite
- **后端**：Tauri 2 + Rust
- **数据库**：SQLite（rusqlite）
- **打包**：Tauri Builder（NSIS 安装包）

## 📝 路线图

- 🖼 题干/选项图片渲染
- 📊 统计可视化（正确率趋势、薄弱题型）
- 📅 每日打卡热力图
- 📤 题库导出为 CSV
- 🏷 章节/知识点标签

完整规划见 [`docs/优化清单.md`](docs/优化清单.md) 和 [`docs/功能缺口清单.md`](docs/功能缺口清单.md)。

## 💝 支持作者

刷题宝是个人业余时间开发的免费软件。如果它帮到了你，可以扫描应用内"支持作者"入口的微信/支付宝二维码支持一下 ❤️

## 📄 License

[MIT](LICENSE)
