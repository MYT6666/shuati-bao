# 刷题宝设计文档

> 把 Word 版题库一键识别转换成可点击选项刷题的桌面应用

## 1. 项目概述

### 1.1 目标
用户上传 Word 版题库（.docx / .doc / 扫描件，含图片公式），应用一键识别题目结构（题干、选项、答案、解析），入库后即可点击选项刷题，并记录答题情况、积累错题本。

### 1.2 核心用户场景
- 个人备考：导入一份题库，反复刷题、攻克错题。
- 多题库管理：同时维护多套题库，分类切换练习。

### 1.3 成功标准
- 能导入混合格式 Word 题库并自动结构化（准确率：AI 模式 >90%，本地 OCR 模式可人工校验修正）。
- 刷题交互流畅，支持单选/多选/判断/填空问答全题型。
- 多题库 + 答题记录 + 错题本可用。
- 日常刷题完全离线，仅导入时按需联网（AI 模式）。

## 2. 技术方案

采用 **Tauri 折中方案**：Tauri 2.x 桌面框架 + 本地 OCR 为主 + AI 大模型可选增强。

### 2.1 技术栈
- **框架**：Tauri 2.x（Rust 后端 + Web 前端）
- **前端**：Vue 3 + TypeScript + Vite + Pinia + Vue Router
- **Word 解析**：`mammoth.js`（前端，.docx → HTML + 图片）；.doc 先用 LibreOffice headless 转 .docx
- **OCR**：Tesseract（`chi_sim` + `eng`），Rust 绑定或调用系统二进制
- **AI 增强**：`reqwest` 调用大模型 API（GLM / 通义千问，用户自带 Key），视觉模型一步到位完成 OCR + 结构化 + 答案关联
- **存储**：SQLite（`rusqlite`），单文件本地数据库
- **图片**：存应用数据目录 `images/`，数据库存相对路径

### 2.2 目录结构
```
src-tauri/              # Rust 后端
  src/
    import/             # Word 解析 + OCR + AI 识别
      docx.rs
      ocr.rs
      ai.rs
      structure.rs      # 结构化解析（题型识别、答案关联）
    db/                 # SQLite 操作（schema、迁移、CRUD）
    commands/           # Tauri 命令（前端调用入口）
  Cargo.toml
  tauri.conf.json
src/                    # Vue 前端
  views/                # 导入向导、题库列表、刷题、错题本、统计、设置
  stores/               # Pinia（题库、刷题、设置）
  components/           # 题目卡片、选项按钮、进度条等
  router/
  App.vue
  main.ts
package.json
vite.config.ts
```

## 3. Word 导入与识别流程

三段式流水线，每段都有降级方案：

### 3.1 文档预处理
- `.docx`：`mammoth.js` 提取 HTML + 内嵌图片
- `.doc`：先用 LibreOffice headless 转 `.docx`，再走上面流程；转换失败提示用户手动转换
- 扫描件 / PDF：直接进入 OCR

### 3.2 内容识别（双引擎，可切换）
**引擎 A（本地，默认）**
- 文本部分：按段落切分
- 图片部分：Tesseract OCR（`chi_sim` + `eng`）

**引擎 B（AI 增强，用户开启并配置 Key）**
- 把整篇文档文本 + 图片发给大模型
- Prompt 要求输出结构化 JSON
- 一步到位：OCR + 结构化 + 答案关联

### 3.3 结构化解析
- **题型识别**（规则引擎）
  - `A. xxx B. xxx` → 选择题（单选/多选按答案数量判定）
  - `正确/错误`、`√/×`、`对/错` → 判断题
  - `___`、`(  )`、`【  】` → 填空题
  - 其余 → 问答
- **答案关联**（自动探测三种模式）
  - 题后紧跟答案（`答案：A`、`【答案】B`）
  - 文末答案表（按题号匹配）
  - AI 模式下由大模型直接关联

### 3.4 人工校验页
导入完成前展示校验表格：
- 表格形式展示识别结果，可编辑修正
- 标记低置信度项（OCR 模糊 / 答案未匹配）标红
- 确认后入库；AI 模式置信度高时可跳过

## 4. 数据模型

SQLite 单文件数据库。

```sql
-- 题库
CREATE TABLE quiz_banks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  description TEXT,
  question_count INTEGER DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

-- 题目
CREATE TABLE questions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
  type TEXT NOT NULL,              -- single / multi / judge / blank
  stem TEXT NOT NULL,              -- 题干（markdown/HTML，支持图片路径）
  options TEXT,                    -- JSON 数组（选择题）
  answer TEXT,                     -- 单选: "A"；多选: ["A","C"]；判断: "true"；填空: ["x","y"]
  analysis TEXT,                   -- 解析
  source_index INTEGER,            -- 原文档题号，便于回溯
  confidence REAL DEFAULT 1.0      -- 识别置信度 0-1
);

-- 答题记录
CREATE TABLE practice_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
  question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  user_answer TEXT,
  is_correct INTEGER NOT NULL,     -- 0/1
  duration_ms INTEGER,
  practiced_at TEXT NOT NULL
);

-- 错题本
CREATE TABLE wrong_questions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  bank_id INTEGER NOT NULL REFERENCES quiz_banks(id) ON DELETE CASCADE,
  question_id INTEGER NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
  wrong_count INTEGER DEFAULT 1,
  last_wrong_at TEXT NOT NULL,
  status TEXT DEFAULT 'pending'    -- pending / mastered
);

-- 设置
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT
);
```

图片存储：题干/选项中的图片存到应用数据目录 `images/<bank_id>/`，数据库只存相对路径。

## 5. 刷题功能与 UI

### 5.1 页面结构（Vue Router）

| 路由 | 页面 | 功能 |
|------|------|------|
| `/` | 题库列表 | 多题库卡片，显示进度、正确率，可新建/导入/删除 |
| `/import` | 导入向导 | 选文件 → 选引擎 → 识别进度 → 校验页 → 入库 |
| `/practice/:bankId` | 刷题页 | 核心交互 |
| `/wrong/:bankId` | 错题本 | 按题库筛选错题，可重练、标记已掌握 |
| `/stats/:bankId` | 统计页 | 正确率趋势、薄弱题型、用时分布 |
| `/settings` | 设置 | AI Key、OCR 引擎、Tesseract 路径、主题 |

### 5.2 刷题页交互
- **顶部**：题库名、进度（12/100）、计时
- **中部**：题干（支持图片渲染）、选项（可点击）
  - 单选：点选项即选中，再点「确认」判分
  - 多选：点多个选项，「确认」判分
  - 判断：√/× 两个大按钮
  - 填空/问答：输入框，「确认」后展示参考答案，自评「答对/答错」
- **答题后**：选项立即反馈对错（绿色对/红色错），解析默认折叠可展开
- **底部**：上一题/下一题、收藏错题、查看解析
- **模式切换**：顺序练习 / 随机练习 / 错题重练 / 模拟考试（限时计分）
- **快捷键**：1-4 选 ABCD，空格确认，←→ 翻页

## 6. 错误处理与边界

| 场景 | 处理 |
|------|------|
| `.doc` 解析失败 | 提示安装 LibreOffice 或手动转 `.docx` |
| Tesseract 未安装 | 首次启动检测，引导下载安装并配置路径 |
| OCR 置信度低（<0.6） | 校验页标红，提示人工核对或切换 AI 引擎重试 |
| 答案关联失败 | 题目入库但 `answer=null`，校验页提示手动补全 |
| AI 调用失败（网络/Key/超时） | 回退本地引擎，提示原因 |
| 大文件（>50 页） | 分批 OCR，进度条反馈，避免界面卡死 |
| 重复导入 | 按题库名 + 题目哈希去重提示 |
| 数据备份 | 每次导入前自动备份数据库到 `backups/`，可回滚 |

## 7. 范围与非目标

### 7.1 本期范围
- 桌面应用（Windows 优先，Mac 可后续适配）
- 单用户本地使用，无账号系统、无云同步
- Word 导入 + 双引擎识别 + 人工校验
- 全题型刷题 + 答题记录 + 错题本 + 基础统计

### 7.2 非目标（后期优化）
- 多用户账号、云同步
- 移动端 / 小程序
- 题库分享市场
- 协作编辑题库
- 本地大模型 OCR（如 PaddleOCR 集成）

## 8. 开放问题（实现期再定）

- AI 服务商默认推荐哪家（GLM vs 通义千问）—— 实现时按视觉能力和价格再定
- Tesseract 分发方式：随应用打包 vs 引导用户安装 —— 打包体积权衡
- 模拟考试的具体计分规则 —— 参考常见考试模式再定
