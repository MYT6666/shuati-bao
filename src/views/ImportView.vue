<template>
  <div
    class="import"
    :class="{ 'drop-active': dragActive }"
    @dragenter.prevent="onDragEnter"
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
  >
    <h2>导入题库</h2>
    <div v-if="bankId" class="target">导入到：{{ bankName }}</div>

    <!-- 拖拽提示覆盖层 -->
    <div v-if="dragActive && step === 1" class="drop-overlay">
      <div class="drop-hint">
        <div class="drop-icon">📂</div>
        <div class="drop-text">松开鼠标导入文件</div>
        <div class="drop-hint-small">支持 .docx / .txt / .md / .pdf</div>
      </div>
    </div>

    <div class="step" v-if="step === 1">
      <h3>步骤1：选择 Word 文件</h3>
      <div class="engine-select">
        <label><input type="radio" v-model="engine" value="local" /> 本地引擎（OCR + 规则解析）</label>
        <label><input type="radio" v-model="engine" value="ai" /> AI 引擎（需配置 API Key）</label>
      </div>
      <button @click="pickFile">选择 .docx 文件</button>
      <p class="hint">仅支持 .docx 格式。如果是 .doc 旧格式，请先用 Word 或 WPS 另存为 .docx；PDF/扫描件暂不支持</p>
      <p v-if="fileName">{{ fileName }}</p>
    </div>

    <div class="step" v-if="step === 2">
      <h3>步骤2：识别中</h3>

      <!-- 阶段指示器 -->
      <div class="stages">
        <div class="stage" :class="stageClass('reading')">
          <span class="stage-icon">{{ stageIcon('reading') }}</span> 读取文件
        </div>
        <div class="stage-line" :class="{ done: stageIndex > 0 }"></div>
        <div class="stage" :class="stageClass('parsing')">
          <span class="stage-icon">{{ stageIcon('parsing') }}</span> 解析文档
        </div>
        <div class="stage-line" :class="{ done: stageIndex > 1 }"></div>
        <div class="stage" :class="stageClass('recognizing')">
          <span class="stage-icon">{{ stageIcon('recognizing') }}</span> {{ engine === 'ai' ? 'AI 识别' : '结构化识别' }}
        </div>
        <div class="stage-line" :class="{ done: stageIndex > 2 }"></div>
        <div class="stage" :class="stageClass('saving')">
          <span class="stage-icon">{{ stageIcon('saving') }}</span> 保存入库
        </div>
      </div>

      <!-- 确定进度条（AI 分块识别阶段） -->
      <div v-if="progress.total > 0" class="progress-bar">
        <div class="progress-fill" :style="{ width: progressPct + '%' }"></div>
        <span class="progress-text">{{ progress.done }} / {{ progress.total }} 块（{{ progressPct }}%）</span>
      </div>

      <!-- 不确定进度条（读取/解析/保存等不可追踪阶段） -->
      <div v-else class="progress-bar indeterminate">
        <div class="progress-fill-indeterminate"></div>
      </div>

      <p class="status-text">{{ status }}</p>
      <p class="elapsed" v-if="elapsed > 0">已耗时 {{ elapsed }} 秒</p>
      <p class="hint">识别中请勿关闭窗口，大题库可能需要 30-60 秒</p>
      <button v-if="engine === 'ai'" class="cancel-btn" @click="cancelImport" :disabled="cancelling">{{ cancelling ? '取消中...' : '取消导入' }}</button>
    </div>

    <div class="step" v-if="step === 2 && importWarning" style="border-color: #f56c6c;">
      <p class="warning-text">{{ importWarning }}</p>
    </div>

    <div class="step" v-if="step === 3">
      <h3>步骤3：校验识别结果（{{ reviewList.length }} 题）</h3>
      <div v-if="importWarning" class="warning-box">{{ importWarning }}</div>
      <ImportReviewTable :list="reviewList" @update="onUpdate" />
      <div class="actions">
        <button @click="confirmImport">确认导入</button>
        <button @click="resetImport">重新选择</button>
      </div>
    </div>

    <div class="step" v-if="step === 4">
      <h3>导入成功！共 {{ importedCount }} 题</h3>
      <button @click="$router.push(`/practice/${bankId}`)">开始刷题</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-dialog'
import { readFile } from '@tauri-apps/plugin-fs'
import { listen } from '@tauri-apps/api/event'
// mammoth 改为动态 import，仅在用户选择文件后才加载（约 400KB 节省首屏）
import { api, Question } from '../utils/api'
import { toastError } from '../utils/toast'
import { useBankStore } from '../stores/bank'
import ImportReviewTable from '../components/ImportReviewTable.vue'

//#region debug-point import-crash-during
// 调试插桩：仅 DEV 环境启用，向本地 debug server 上报崩溃点
const DBG_URL = 'http://127.0.0.1:8899/logs'
const DBG_SESSION = 'import-crash-during'
const DBG_ENABLED = import.meta.env.DEV
let dbgSeq = 0
function dbg(stage: string, data: Record<string, unknown> = {}) {
  if (!DBG_ENABLED) return
  dbgSeq++
  const payload = JSON.stringify({
    ts: Date.now(),
    level: 'info',
    sessionId: DBG_SESSION,
    stage,
    seq: dbgSeq,
    data
  })
  try {
    fetch(DBG_URL, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: payload,
      keepalive: true
    }).catch(() => {})
  } catch (e) { /* ignore */ }
}
//#endregion

const route = useRoute()
const router = useRouter()
const bankStore = useBankStore()
const bankId = ref(Number(route.params.bankId) || 0)
const bankName = ref('')
const step = ref(1)
const fileName = ref('')
const status = ref('')
const reviewList = ref<Question[]>([])
const importedCount = ref(0)
const engine = ref<'local' | 'ai'>('local')
const progress = ref({ done: 0, total: 0 })
const importWarning = ref('')
const cancelling = ref(false)

// 拖拽上传
const dragActive = ref(false)
let dragCounter = 0
function onDragEnter() {
  if (step.value !== 1) return
  dragCounter++
  dragActive.value = true
}
function onDragOver(e: DragEvent) {
  if (step.value !== 1) return
  e.dataTransfer!.dropEffect = 'copy'
}
function onDragLeave() {
  dragCounter = Math.max(0, dragCounter - 1)
  if (dragCounter === 0) dragActive.value = false
}
async function onDrop(e: DragEvent) {
  dragActive.value = false
  dragCounter = 0
  if (step.value !== 1) return
  const file = e.dataTransfer?.files?.[0]
  if (!file) return
  // Tauri 2 拖拽：dataTransfer.files 中可拿到 path
  const filePath = (file as any).path as string | undefined
  if (!filePath) {
    toastError('无法获取文件路径，请用"选择文件"按钮')
    return
  }
  const ext = (filePath.split('.').pop() || '').toLowerCase()
  if (!['docx', 'txt', 'md', 'pdf'].includes(ext)) {
    toastError('不支持的文件格式：' + ext + '（仅支持 .docx / .txt / .md / .pdf）')
    return
  }
  // 模拟选择文件，复用 pickFile 之后的流程
  fileName.value = filePath
  await runImport(filePath)
}

// 进度阶段
type Stage = 'reading' | 'parsing' | 'recognizing' | 'saving' | 'done'
const stage = ref<Stage>('reading')
const stageOrder: Stage[] = ['reading', 'parsing', 'recognizing', 'saving', 'done']
const stageIndex = computed(() => stageOrder.indexOf(stage.value))
const progressPct = computed(() => progress.value.total > 0 ? Math.round(progress.value.done / progress.value.total * 100) : 0)

function stageClass(s: Stage) {
  const idx = stageOrder.indexOf(s)
  return { active: stage.value === s, done: stageIndex.value > idx }
}
function stageIcon(s: Stage) {
  const idx = stageOrder.indexOf(s)
  if (stageIndex.value > idx) return '✓'
  if (stage.value === s) return '⟳'
  return '○'
}

// 计时器
const elapsed = ref(0)
let timerId: number | null = null
function startTimer() {
  elapsed.value = 0
  if (timerId) window.clearInterval(timerId)
  timerId = window.setInterval(() => { elapsed.value++ }, 1000)
}
function stopTimer() {
  if (timerId) { window.clearInterval(timerId); timerId = null }
}

// BUG-007 修复：将 AI 进度监听器提升为组件级，组件卸载时统一清理
// 旧实现中 unlisten 是 runImport 内的局部变量，若用户在 AI 导入过程中离开页面，
// 监听器不会被释放，导致内存泄漏与对已销毁组件的状态更新
let aiProgressUnlisten: (() => void) | null = null
function clearAiProgressListener() {
  if (aiProgressUnlisten) {
    try { aiProgressUnlisten() } catch (_) { /* ignore */ }
    aiProgressUnlisten = null
  }
}

onUnmounted(() => {
  stopTimer()
  clearAiProgressListener()
})

onMounted(async () => {
  if (!bankId.value) {
    const name = prompt('请输入题库名称')
    if (name) {
      try {
        const b = await bankStore.create(name, null)
        bankId.value = b.id
        bankName.value = b.name
      } catch (e) {
      toastError('创建题库失败：' + (e instanceof Error ? e.message : String(e)))
      router.push('/')
    }
    } else {
      router.push('/')
    }
  } else {
    bankName.value = bankStore.banks.find(b => b.id === bankId.value)?.name || ''
  }
})

function htmlToText(html: string): string {
  // P0-3: 用 DOMParser 替代 innerHTML，避免 <img onerror=...> 等 XSS 风险
  // parseToString('text/html') 不执行脚本/不触发内联事件
  const doc = new DOMParser().parseFromString(html, 'text/html')
  return doc.body.textContent || doc.body.innerText || ''
}

async function pickFile() {
  dbg('pickFile_start', { engine: engine.value, bankId: bankId.value })
  const selected = await open({
    filters: [
      { name: '题库文件', extensions: ['docx', 'txt', 'md', 'pdf'] },
    ],
  })
  if (!selected || Array.isArray(selected)) return
  const filePath = selected as string
  fileName.value = filePath
  await runImport(filePath)
}

// 实际导入流程：传入文件绝对路径
async function runImport(filePath: string) {
  const ext = (filePath.split('.').pop() || '').toLowerCase()
  step.value = 2
  progress.value = { done: 0, total: 0 }
  importWarning.value = ''
  cancelling.value = false
  startTimer()

  // 监听 AI 进度事件
  // BUG-007 修复：复用组件级 aiProgressUnlisten，避免泄漏
  clearAiProgressListener()
  if (engine.value === 'ai') {
    aiProgressUnlisten = await listen<{ done: number; total: number }>('ai_progress', (e) => {
      progress.value = e.payload
      if (e.payload.total > 0) {
        status.value = `AI 识别中... 第 ${e.payload.done}/${e.payload.total} 块`
      }
    })
  }

  try {
    // AI 引擎：先测试连通性，避免卡死
    if (engine.value === 'ai') {
      stage.value = 'reading'
      status.value = '正在测试 AI 连接...'
      dbg('ai_connection_test_start')
      try {
        await api.testAiConnection()
        dbg('ai_connection_test_ok')
      } catch (e) {
        dbg('ai_connection_test_fail', { err: e instanceof Error ? e.message : String(e) })
        throw new Error('AI 连接失败：' + (e instanceof Error ? e.message : String(e)) + '。请到设置页检查 API Key、地址、模型。')
      }
    }

    // 阶段1+2：按文件类型分别处理
    let html: string
    let text: string
    stage.value = 'reading'
    if (ext === 'docx') {
      // Word 文档：动态加载 mammoth
      status.value = '正在读取 Word 文件...'
      dbg('readFile_start', { path: filePath })
      const bytes = await readFile(filePath)
      const arrayBuffer = bytes.buffer
      dbg('readFile_done', { bytes: bytes.length })
      stage.value = 'parsing'
      status.value = '正在解析 Word 文档...'
      dbg('mammoth_start')
      const mammoth = (await import('mammoth')).default
      const result = await mammoth.convertToHtml({ arrayBuffer })
      html = result.value
      dbg('mammoth_done', { htmlLen: html.length, messages: result.messages.length })
      text = htmlToText(html)
    } else if (ext === 'pdf') {
      // PDF：直接传路径给后端 lopdf 处理
      status.value = '正在解析 PDF...'
      dbg('pdf_start', { path: filePath })
      const cnt = await api.importFromPdf(bankId.value, filePath)
      dbg('pdf_done', { count: cnt })
      // 跳到 review 阶段
      stage.value = 'saving'
      status.value = '正在加载题目...'
      const qs = await api.listQuestions(bankId.value)
      reviewList.value = qs
      stage.value = 'done'
      stopTimer()
      importWarning.value = cnt === 0 ? '未识别到题目，请检查 PDF 是否含可选中文本（扫描件无法识别）。' : ''
      clearAiProgressListener()
      return
    } else {
      // txt / md 等纯文本
      status.value = '正在读取文本...'
      dbg('readText_start', { path: filePath })
      const bytes = await readFile(filePath)
      text = new TextDecoder('utf-8').decode(bytes)
      // 转成 mammoth 兼容 HTML：每个非空行包 <p>，后端会复用 html_to_questions
      html = text
        .split(/\r?\n/)
        .map(l => l.trim())
        .filter(l => l.length > 0)
        .map(l => `<p>${l.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')}</p>`)
        .join('\n')
      dbg('readText_done', { textLen: text.length, htmlLen: html.length })
    }

    // 阶段3：识别
    stage.value = 'recognizing'
    status.value = engine.value === 'ai' ? 'AI 识别中（分块并发处理）...' : '结构化识别中...'
    if (engine.value === 'ai') {
      dbg('importWithAi_call_start', { textLen: text.length })
      const importResult = await api.importWithAi(bankId.value, text)
      dbg('importWithAi_call_done', { count: importResult.count, expected: importResult.expected })
      if (cancelling.value) {
        importWarning.value = `已取消导入，仅识别到 ${importResult.count} 题（预计 ${importResult.expected} 题）。可重新选择文件再次导入。`
        cancelling.value = false
      } else if (importResult.expected > 0 && importResult.count < importResult.expected - 5) {
        importWarning.value = `⚠ 识别到 ${importResult.count} 题，但文档预估约 ${importResult.expected} 题，可能有 ${importResult.expected - importResult.count} 题丢失（AI 输出截断或网络错误）。可尝试重新导入。`
      }
    } else {
      dbg('importFromHtml_call_start', { htmlLen: html.length })
      const cnt = await api.importFromHtml(bankId.value, html)
      dbg('importFromHtml_call_done', { count: cnt })
    }

    // 阶段4：保存入库
    stage.value = 'saving'
    status.value = '正在保存到数据库...'
    dbg('listQuestions_call_start', { bankId: bankId.value })
    const qs = await api.listQuestions(bankId.value)
    dbg('listQuestions_call_done', { count: qs.length })
    dbg('reviewList_assign_start')
    reviewList.value = qs
    dbg('reviewList_assign_done')

    stage.value = 'done'
    stopTimer()
    dbg('step3_switch_start')
    step.value = 3
    dbg('step3_switch_done')
    clearAiProgressListener()
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    dbg('pickFile_error', { msg, cancelling: cancelling.value, stack: e instanceof Error ? e.stack : undefined })
    if (cancelling.value) {
      status.value = '已取消导入'
    } else {
      status.value = '失败：' + msg
    }
    step.value = 1
    stopTimer()
    cancelling.value = false
  } finally {
    clearAiProgressListener()
  }
}

async function cancelImport() {
  cancelling.value = true
  try {
    await api.cancelImport()
    status.value = '正在取消...'
  } catch (e) {
    console.error('取消失败：', e)
  }
}

function onUpdate(q: Question) {
  const i = reviewList.value.findIndex(x => x.id === q.id)
  if (i >= 0) reviewList.value[i] = q
}

async function resetImport() {
  if (bankId.value && reviewList.value.length > 0) {
    try {
      await api.clearBankQuestions(bankId.value)
    } catch (e) {
      console.error('清理已导入题目失败：', e)
    }
  }
  reviewList.value = []
  fileName.value = ''
  importWarning.value = ''
  step.value = 1
}

async function confirmImport() {
  importedCount.value = reviewList.value.length
  step.value = 4
}
</script>

<style scoped>
.import { position: relative; min-height: 100%; }
.drop-active { outline: 2px dashed var(--color-primary); outline-offset: -8px; }
.drop-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.55); display: flex; align-items: center; justify-content: center; z-index: 1000; pointer-events: none; animation: dropFade 0.15s; }
.drop-hint { background: var(--color-card); border-radius: var(--radius-lg); padding: 48px 80px; text-align: center; box-shadow: 0 20px 60px rgba(0,0,0,0.3); border: 3px dashed var(--color-primary); }
.drop-icon { font-size: 64px; margin-bottom: 12px; }
.drop-text { font-size: 20px; font-weight: 600; color: var(--color-text); margin-bottom: 6px; }
.drop-hint-small { font-size: 13px; color: var(--color-text-tertiary); }
@keyframes dropFade { from { opacity: 0; } to { opacity: 1; } }

.step { background: var(--color-card); border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: 24px; margin-top: 16px; }
.actions { margin-top: 16px; display: flex; gap: 8px; }
button { padding: 8px 16px; border: 1px solid var(--color-border); border-radius: var(--radius-md); cursor: pointer; background: var(--color-card); color: var(--color-text); }
button:hover { background: var(--color-border-light); }
.target { color: var(--color-text-secondary); margin-bottom: 12px; }
.engine-select { margin-bottom: 12px; }
.engine-select label { display: block; margin: 4px 0; }
.hint { color: var(--color-text-tertiary); font-size: 13px; }
.status-text { font-size: 15px; margin: 12px 0 4px; }
.elapsed { color: var(--color-text-tertiary); font-size: 13px; margin: 4px 0; }

/* 阶段指示器 */
.stages { display: flex; align-items: center; margin: 16px 0; flex-wrap: wrap; gap: 4px; }
.stage { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--color-text-tertiary); padding: 4px 8px; border-radius: var(--radius-sm); white-space: nowrap; }
.stage.active { color: var(--color-primary); font-weight: 500; background: var(--color-primary-light); }
.stage.done { color: var(--color-primary); }
.stage-icon { font-size: 16px; }
.stage.active .stage-icon { animation: spin 1s linear infinite; display: inline-block; }
.stage-line { width: 20px; height: 2px; background: var(--color-border); }
.stage-line.done { background: var(--color-primary); }

@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

/* 确定进度条 */
.progress-bar { position: relative; width: 100%; max-width: 500px; height: 28px; background: var(--color-border-light); border-radius: 14px; overflow: hidden; margin: 12px 0; }
.progress-fill { position: absolute; left: 0; top: 0; height: 100%; background: var(--color-primary); transition: width 0.3s; }
.progress-text { position: absolute; width: 100%; text-align: center; line-height: 28px; font-size: 13px; color: var(--color-text); font-weight: 500; }

/* 不确定进度条（条纹流动动画） */
.progress-bar.indeterminate { background: var(--color-border-light); }
.progress-fill-indeterminate { position: absolute; height: 100%; width: 40%; background: var(--color-primary); border-radius: 14px; animation: indeterminate 1.5s ease-in-out infinite; }
@keyframes indeterminate {
  0% { left: -40%; }
  100% { left: 100%; }
}

.warning-box { background: var(--color-warning-light); border: 1px solid var(--color-warning); border-radius: var(--radius-md); padding: 12px; margin-bottom: 12px; color: var(--color-warning); font-size: 14px; }
.warning-text { color: var(--color-danger); font-size: 14px; margin: 0; }
.cancel-btn { margin-top: 12px; padding: 8px 20px; border: 1px solid var(--color-danger); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-danger); cursor: pointer; font-size: 14px; }
.cancel-btn:hover { background: var(--color-danger-light); }
.cancel-btn:disabled { color: var(--color-text-tertiary); border-color: var(--color-border); cursor: not-allowed; }
</style>
