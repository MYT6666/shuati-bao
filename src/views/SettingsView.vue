<template>
  <div class="settings">
    <h2>设置</h2>

    <section>
      <h3>外观</h3>
      <label>主题
        <select v-model="theme" @change="saveTheme">
          <option value="system">跟随系统</option>
          <option value="light">亮色</option>
          <option value="dark">暗色</option>
        </select>
      </label>
      <label>字号
        <select v-model="fontSize" @change="saveFontSize">
          <option value="small">小</option>
          <option value="medium">中（默认）</option>
          <option value="large">大</option>
        </select>
      </label>
      <p class="hint">字号调整界面整体大小，方便不同视力需求</p>
    </section>

    <section>
      <h3>AI 识别引擎</h3>
      <label>API Key <input v-model="apiKey" @blur="save('ai_api_key', apiKey)" type="password" placeholder="粘贴你的 API Key" /></label>
      <p class="hint">没有 Key？去以下平台注册即可获取（都送免费额度）：</p>
      <div class="key-links">
        <a href="https://open.bigmodel.cn/" target="_blank">智谱GLM → 注册送免费额度，glm-4-flash 免费</a>
        <a href="https://platform.deepseek.com/" target="_blank">DeepSeek → 注册送 500 万 token</a>
        <a href="https://agnes-ai.com/" target="_blank">Agnes → 免费高并发</a>
      </div>
      <label>Base URL <input v-model="baseUrl" @blur="save('ai_base_url', baseUrl)" /></label>
      <label>模型 <input v-model="model" @blur="save('ai_model', model)" /></label>
      <div class="model-tips">
        <p class="hint">推荐模型（点击可直接填入）：</p>
        <div class="model-list">
          <button class="model-pick" @click="pickModel('glm-4-flash', 'https://open.bigmodel.cn/api/paas/v4')">
            <strong>glm-4-flash</strong> <span class="tag fast">极速·免费</span> 智谱
          </button>
          <button class="model-pick" @click="pickModel('deepseek-chat', 'https://api.deepseek.com/v1')">
            <strong>deepseek-chat</strong> <span class="tag fast">快</span> DeepSeek
          </button>
          <button class="model-pick" @click="pickModel('agnes-2.0-flash', 'https://api.agnes-ai.com/v1')">
            <strong>agnes-2.0-flash</strong> <span class="tag fast">快·免费</span> Agnes
          </button>
          <button class="model-pick" @click="pickModel('gpt-4o-mini', 'https://api.openai.com/v1')">
            <strong>gpt-4o-mini</strong> <span class="tag fast">快</span> OpenAI
          </button>
        </div>
        <p class="warn">⚠ 不要用 glm-4v / gpt-4o 等视觉模型，纯文本任务用视觉模型会慢 3-5 倍</p>
      </div>
      <div class="test-row">
        <button class="test-btn" :disabled="testing" @click="testConnection">
          {{ testing ? '测试中...' : '测试连接' }}
        </button>
        <span v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'fail'">
          {{ testResult.ok ? '✓ 连接成功' : '✗ ' + testResult.msg }}
        </span>
      </div>
    </section>

    <section>
      <h3>OCR 引擎</h3>
      <p>{{ ocrAvailable ? '✓ Tesseract 可用' : '✗ 未检测到 Tesseract，图片识别将不可用' }}</p>
      <p class="hint">当前版本仅支持文本题（含图示/公式的题目需要后续版本支持）</p>
    </section>

    <section>
      <h3>更新</h3>
      <p class="hint">当前版本：<b>{{ currentVersion }}</b></p>
      <p class="hint">应用启动时会自动检查更新；也可手动点击下方按钮</p>
      <div class="data-actions">
        <button class="data-btn" :disabled="checkingUpdate" @click="manualCheckUpdate">
          {{ checkingUpdate ? '检查中...' : '🔍 检查更新' }}
        </button>
        <button class="data-btn" @click="showUpdateLog = !showUpdateLog">📜 更新日志</button>
      </div>
      <div v-if="showUpdateLog" class="update-log">
        <h4>更新日志</h4>
        <div class="log-entry">
          <span class="log-version">v0.1.8</span>
          <ul>
            <li>修复 mammoth 提取 docx 时选项分隔符错位导致部分题目选项被并入题干的问题（如习思想2第101题）</li>
            <li>PDF 导入改为异步执行，支持取消按钮和逐页进度反馈</li>
            <li>PDF 导入添加 5 分钟硬超时保护，单页失败自动跳过</li>
            <li>答案区识别优化，第十五章等缺少小节标题的章节可基于内容自动推断题型</li>
          </ul>
        </div>
        <div class="log-entry">
          <span class="log-version">v0.1.1</span>
          <ul>
            <li>更新应用图标（全平台）</li>
            <li>修复 WebView 缓存导致旧图标不刷新的问题</li>
            <li>修复 exe 文件图标不显示的问题（多尺寸 ICO 嵌入）</li>
          </ul>
        </div>
        <div class="log-entry">
          <span class="log-version">v0.1.0</span>
          <ul>
            <li>首次发布</li>
            <li>支持 docx / txt / md / pdf 导入</li>
            <li>AI 自动解析（多模型：智谱 / DeepSeek / Agnes）</li>
            <li>收藏夹 / 错题本 / 学习热力图</li>
            <li>题目导航 / 搜索高亮 / 题型筛选</li>
            <li>自动更新支持</li>
          </ul>
        </div>
      </div>
    </section>

    <section>
      <h3>数据</h3>
      <p class="hint" v-if="dbInfo">
        <span class="data-label">数据库位置：</span>
        <code class="data-path">{{ dbInfo.path }}</code>
        <span class="data-size">({{ formatSize(dbInfo.size_bytes) }})</span>
      </p>
      <p class="hint" v-else>加载中...</p>
      <p class="hint" v-if="dbInfo">
        <span class="data-label">备份目录：</span>
        <code class="data-path">{{ dbInfo.backups_dir }}</code>
        <span class="data-size">({{ dbInfo.backup_count }} 个备份)</span>
      </p>
      <div class="data-actions">
        <button class="data-btn" @click="backupDb">💾 立即备份</button>
        <button class="data-btn" @click="openFolder">📁 打开数据目录</button>
        <button class="data-btn" @click="showCustomize = !showCustomize">⚙️ 自定义位置</button>
      </div>
      <div v-if="showCustomize" class="customize-box">
        <p class="hint">输入新的数据库文件目录（必须是已存在的文件夹路径）：</p>
        <div class="customize-row">
          <input v-model="customDir" placeholder="例如 D:\MyData\刷题宝" class="dir-input" />
          <button class="data-btn" :disabled="applying" @click="pickCustomDir">📂 选择文件夹</button>
          <button class="data-btn primary" :disabled="!customDir.trim() || applying" @click="applyCustomDir">应用</button>
        </div>
        <p class="hint warn">⚠ 修改后需点"重启应用"生效；当前数据库会被复制到新位置。已存在则覆盖。</p>
        <p class="hint success" v-if="applyResult">{{ applyResult }}</p>
      </div>
    </section>

    <section class="donate-section">
      <h3>支持作者</h3>
      <p class="donate-intro">
        刷题宝是个人业余时间开发的免费软件。
        如果它帮到了你，可以支持一下作者 💝
      </p>
      <button class="donate-cta" @click="showDonate = true">
        <span class="coffee">💝</span> 支持作者
      </button>
    </section>

    <section class="feedback-section">
      <h3>意见反馈</h3>
      <p class="hint">
        遇到 Bug？想要新功能？有其他建议？<br />
        点击下方按钮告诉我们，反馈会直接发到作者手里。
      </p>
      <button class="feedback-cta" @click="showFeedback = true">
        <span class="envelope">💌</span> 提交意见反馈
      </button>
    </section>

    <DonateDialog :visible="showDonate" @close="showDonate = false" />
    <FeedbackDialog :visible="showFeedback" @close="showFeedback = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { check as checkUpdateRaw, promptAndApplyUpdate } from '../utils/updater'
import { api } from '../utils/api'
import { toastSuccess, toastError } from '../utils/toast'
import DonateDialog from '../components/DonateDialog.vue'
import FeedbackDialog from '../components/FeedbackDialog.vue'

const currentVersion = ref('0.0.0')
const checkingUpdate = ref(false)
const showUpdateLog = ref(false)

const apiKey = ref('')
const baseUrl = ref('https://open.bigmodel.cn/api/paas/v4')
const model = ref('glm-4-flash')
const ocrAvailable = ref(false)
const testing = ref(false)
const testResult = ref<{ ok: boolean; msg: string } | null>(null)
const theme = ref('system')
const fontSize = ref('medium')
const dbInfo = ref<{ path: string; size_bytes: number; backups_dir: string; backup_count: number } | null>(null)
const showDonate = ref(false)
const showFeedback = ref(false)
const showCustomize = ref(false)
const customDir = ref('')
const applying = ref(false)
const applyResult = ref('')
const applyRestart = ref(false)

function formatSize(b: number): string {
  if (b < 1024) return `${b} B`
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`
  if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(2)} MB`
  return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`
}

async function openFolder() {
  try {
    await api.openDbFolder()
  } catch (e) {
    toastError('打开失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function pickCustomDir() {
  try {
    const selected = await api.pickDatabaseFolder()
    if (selected) {
      customDir.value = selected
    }
  } catch (e) {
    console.error('选择文件夹失败：', e)
    toastError('选择文件夹失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

// @ts-ignore
async function applyCustomDir() {
  if (!customDir.value.trim()) return
  applying.value = true
  applyResult.value = ''
  applyRestart.value = false
  try {
    const newPath = await api.changeDbPath(customDir.value.trim())
    applyResult.value = '✓ 已设置，新位置：' + newPath + '\n点左下角"重启应用"按钮立即生效。'
    applyRestart.value = true
    // 刷新显示
    dbInfo.value = await api.getDbInfo()
  } catch (e) {
    applyResult.value = '✗ 失败：' + (e instanceof Error ? e.message : String(e))
  } finally {
    applying.value = false
  }
}

onMounted(async () => {
  try {
    apiKey.value = (await api.getSetting('ai_api_key')) || ''
    baseUrl.value = (await api.getSetting('ai_base_url')) || baseUrl.value
    model.value = (await api.getSetting('ai_model')) || model.value
    ocrAvailable.value = await api.ocrAvailable()
    // 读取主题/字号设置
    theme.value = (await api.getSetting('ui_theme')) || 'system'
    fontSize.value = (await api.getSetting('ui_font_size')) || 'medium'
    applyTheme()
    applyFontSize()
    // 读取真实数据库信息（不再依赖 db_path 设置项）
    try {
      dbInfo.value = await api.getDbInfo()
    } catch (e) {
    console.error('获取数据库信息失败：', e)
    }
    // 读取应用版本
    try {
      currentVersion.value = await getVersion()
    } catch (e) {
      console.error('读取版本失败：', e)
    }
  } catch (e) {
    toastError('加载设置失败：' + (e instanceof Error ? e.message : String(e)))
  }
})

// 手动检查更新
async function manualCheckUpdate() {
  checkingUpdate.value = true
  try {
    const update = await checkUpdateRaw()
    if (!update) {
      toastSuccess('已是最新版本')
      return
    }
    // 有新版本，弹窗下载
    await promptAndApplyUpdate(update)
  } catch (e) {
    toastError('检查更新失败：' + (e instanceof Error ? e.message : String(e)))
  } finally {
    checkingUpdate.value = false
  }
}

function applyTheme() {
  const html = document.documentElement
  if (theme.value === 'light') {
    html.setAttribute('data-theme', 'light')
  } else if (theme.value === 'dark') {
    html.setAttribute('data-theme', 'dark')
  } else {
    html.removeAttribute('data-theme')
  }
}

function applyFontSize() {
  document.documentElement.setAttribute('data-font-size', fontSize.value)
}

async function saveTheme() {
  applyTheme()
  await api.setSetting('ui_theme', theme.value)
}

async function saveFontSize() {
  applyFontSize()
  await api.setSetting('ui_font_size', fontSize.value)
}

async function save(key: string, value: string) {
  try {
    await api.setSetting(key, value)
    testResult.value = null
  } catch (e) {
    toastError('保存失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function pickModel(m: string, url: string) {
  model.value = m
  baseUrl.value = url
  await save('ai_model', m)
  await save('ai_base_url', url)
}

async function testConnection() {
  testing.value = true
  testResult.value = null
  try {
    await api.testAiConnection()
    testResult.value = { ok: true, msg: '' }
  } catch (e) {
    testResult.value = { ok: false, msg: e instanceof Error ? e.message : String(e) }
  } finally {
    testing.value = false
  }
}

async function backupDb() {
  try {
    const dst = await api.backupDatabase()
    toastSuccess('备份成功：' + dst)
  } catch (e) {
    toastError('备份失败：' + (e instanceof Error ? e.message : String(e)))
  }
}
</script>

<style scoped>
section { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 16px; margin-bottom: 16px; }
label { display: block; margin: 8px 0; }
input, select { padding: 6px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); width: 320px; background: var(--color-card); color: var(--color-text); }
.hint { color: var(--color-text-tertiary); font-size: 13px; }
.test-row { margin-top: 12px; display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.test-btn { padding: 6px 18px; border: 1px solid var(--color-primary); border-radius: var(--radius-md); background: var(--color-primary); color: #fff; cursor: pointer; font-size: 14px; }
.test-btn:disabled { background: var(--color-text-tertiary); border-color: var(--color-text-tertiary); cursor: not-allowed; }
.test-result { font-size: 14px; }
.test-result.ok { color: var(--color-success); }
.test-result.fail { color: var(--color-danger); word-break: break-all; }
.model-tips { margin-top: 8px; }
.model-list { display: flex; flex-direction: column; gap: 6px; margin: 8px 0; }
.model-pick { text-align: left; padding: 8px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg); cursor: pointer; font-size: 13px; transition: border-color 0.2s; color: var(--color-text); }
.model-pick:hover { border-color: var(--color-primary); background: var(--color-primary-light); }
.tag { display: inline-block; padding: 1px 6px; border-radius: 3px; font-size: 11px; margin: 0 4px; }
.tag.fast { background: var(--color-success-light); color: var(--color-success); }
.warn { color: #e65100; font-size: 13px; margin-top: 4px; }
.key-links { display: flex; flex-direction: column; gap: 4px; margin: 8px 0; }
.key-links a { color: var(--color-primary); font-size: 13px; text-decoration: none; }
.key-links a:hover { text-decoration: underline; }
.data-btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); cursor: pointer; transition: background 0.15s; }
.data-btn:hover { background: var(--color-border-light); }
.data-btn.primary { background: var(--color-primary); color: #fff; border-color: var(--color-primary); }
.data-btn.primary:hover { background: var(--color-primary-dark); }
.data-btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
.data-btn.danger { color: #e53935; border-color: #e53935; }
.data-btn.danger:hover { background: #e53935; color: #fff; }
.data-actions { display: flex; gap: 8px; margin: 8px 0; flex-wrap: wrap; }
.data-label { color: var(--color-text-secondary); margin-right: 4px; }
.data-path { background: var(--color-bg); padding: 2px 6px; border-radius: 4px; font-family: ui-monospace, Consolas, monospace; font-size: 12px; color: var(--color-text); border: 1px solid var(--color-border-light); word-break: break-all; }
.data-size { color: var(--color-text-tertiary); margin-left: 6px; font-size: 12px; }
.customize-box { background: var(--color-bg); border: 1px solid var(--color-border-light); border-radius: var(--radius-md); padding: 12px; margin-top: 8px; }
.customize-row { display: flex; gap: 8px; margin: 8px 0; }
.dir-input { flex: 1; padding: 6px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); font-size: 13px; }
.hint.warn { color: var(--color-warning); }
.hint.success { color: var(--color-success); }

/* 更新日志 */
.update-log { background: var(--color-bg); border: 1px solid var(--color-border-light); border-radius: var(--radius-md); padding: 12px 16px; margin-top: 8px; max-height: 280px; overflow-y: auto; }
.update-log h4 { margin: 0 0 8px 0; font-size: 13px; color: var(--color-text-secondary); }
.log-entry { margin-bottom: 8px; }
.log-version { display: inline-block; padding: 2px 8px; background: var(--color-primary-light); color: var(--color-primary); border-radius: 4px; font-size: 12px; font-weight: 600; margin-bottom: 4px; }
.update-log ul { margin: 4px 0 0 0; padding-left: 20px; color: var(--color-text-secondary); font-size: 13px; line-height: 1.7; }

.donate-section { text-align: center; }
.donate-section h3 { margin-top: 0; }
.donate-intro {
  color: var(--color-text-secondary);
  font-size: 14px;
  line-height: 1.6;
  margin: 8px 0 16px;
}
.donate-cta {
  display: inline-flex; align-items: center; gap: 8px;
  padding: 10px 24px;
  background: linear-gradient(135deg, #ff9a56, #ff6a88);
  border: none;
  border-radius: 24px;
  color: #fff;
  font-size: 15px;
  font-weight: 500;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(255, 106, 136, 0.3);
  transition: all 0.2s;
}
.donate-cta:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(255, 106, 136, 0.4);
}
.donate-cta .coffee { font-size: 18px; }
</style>
