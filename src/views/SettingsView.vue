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
      <h3>数据</h3>
      <p class="hint">数据库位置：{{ dbPath || '加载中...' }}</p>
      <button class="data-btn" @click="backupDb">立即备份数据库</button>
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

    <DonateDialog :visible="showDonate" @close="showDonate = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../utils/api'
import DonateDialog from '../components/DonateDialog.vue'

const apiKey = ref('')
const baseUrl = ref('https://open.bigmodel.cn/api/paas/v4')
const model = ref('glm-4-flash')
const ocrAvailable = ref(false)
const testing = ref(false)
const testResult = ref<{ ok: boolean; msg: string } | null>(null)
const theme = ref('system')
const fontSize = ref('medium')
const dbPath = ref('')
const showDonate = ref(false)

onMounted(async () => {
  try {
    apiKey.value = (await api.getSetting('ai_api_key')) || ''
    baseUrl.value = (await api.getSetting('ai_base_url')) || baseUrl.value
    model.value = (await api.getSetting('ai_model')) || model.value
    ocrAvailable.value = await api.ocrAvailable()
    // 读取主题/字号设置
    theme.value = (await api.getSetting('ui_theme')) || 'system'
    fontSize.value = (await api.getSetting('ui_font_size')) || 'medium'
    dbPath.value = (await api.getSetting('db_path')) || ''
    applyTheme()
    applyFontSize()
  } catch (e) {
    alert('加载设置失败：' + (e instanceof Error ? e.message : String(e)))
  }
})

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
    alert('保存失败：' + (e instanceof Error ? e.message : String(e)))
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
    await api.backupDatabase()
    alert('备份成功！已保存到应用数据目录的 backups/ 文件夹')
  } catch (e) {
    alert('备份失败：' + (e instanceof Error ? e.message : String(e)))
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
.data-btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); cursor: pointer; }
.data-btn:hover { background: var(--color-border-light); }

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
