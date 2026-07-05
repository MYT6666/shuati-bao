<template>
  <div class="practice">
    <!-- 第一排：题库名 + 进度条 + 模式 + 倒计时 -->
    <div class="topbar main-bar">
      <div class="main-left">
        <span class="bank-name">{{ bankName }}</span>
        <span v-if="mode === 'exam'" class="exam-timer" :class="{ 'time-up': examTimeUp }">
          ⏱ {{ formatExamTime(examRemainingSecs) }}
        </span>
      </div>
      <div class="main-progress">
        <span class="progress-text">{{ current + 1 }} / {{ questions.length }}</span>
        <div class="progress-track">
          <div class="progress-fill" :style="{ width: ((current + 1) / Math.max(questions.length, 1) * 100) + '%' }"></div>
        </div>
      </div>
      <div class="main-right">
        <select v-model="mode" class="mode-select">
          <option value="order">顺序练习</option>
          <option value="random">随机练习</option>
          <option value="wrong">错题重练</option>
          <option value="exam">模拟考试</option>
        </select>
        <button class="help-btn" title="快捷键帮助 (?)" @click="showHelp = true">?</button>
      </div>
    </div>

    <!-- 第二排：搜索 / 类型筛选 / 自动下一题 / 重置 / 交卷 -->
    <div class="topbar tool-bar">
      <div class="search-box">
        <input v-model="searchQuery" placeholder="搜索题目..." @keyup.enter="doSearch" />
        <button v-if="searchQuery" class="search-btn" @click="doSearch">搜索</button>
        <button v-if="searchResults" class="clear-search-btn" @click="clearSearch">✕</button>
      </div>
      <div class="type-filter" v-if="availableTypes.length > 1">
        <span class="filter-label">题型:</span>
        <button
          v-for="t in availableTypes"
          :key="t"
          class="type-chip-btn"
          :class="{ active: !typeFilter.includes(t) }"
          @click="toggleTypeFilter(t)"
        >{{ t }}</button>
      </div>
      <label v-if="mode !== 'exam'" class="auto-next-toggle">
        <input type="checkbox" v-model="autoNext" />
        答对自动下一题
      </label>
      <button class="restart-btn" @click="restart">从头开始</button>
      <button v-if="mode === 'exam' && !examSubmitted" class="submit-exam-btn" @click="submitExam">交卷</button>
    </div>

    <div v-if="searchResults" class="search-result-banner">
      搜索到 <b>{{ searchResults.length }}</b> 道包含"<b>{{ searchQuery }}</b>"的题目，点击跳转
      <button class="close-btn" @click="clearSearch">✕</button>
    </div>
    <div v-if="searchResults && searchResults.length" class="search-result-list">
      <div
        v-for="(r, ri) in searchResults.slice(0, 30)"
        :key="r.id"
        class="search-result-item"
        @click="jumpToQuestion(r.id)"
      >
        <span class="search-result-idx">{{ ri + 1 }}.</span>
        <span class="search-result-text" v-html="highlightText(r.stem, searchQuery)"></span>
        <span class="search-result-types">
          <span class="type-chip">{{ r.type }}</span>
        </span>
      </div>
      <div v-if="searchResults.length > 30" class="search-result-overflow">仅显示前 30 条，共 {{ searchResults.length }} 条匹配</div>
    </div>

    <!-- 快捷键帮助浮窗 -->
    <Teleport to="body">
      <div v-if="showHelp" class="help-overlay" @click.self="showHelp = false">
        <div class="help-modal">
          <div class="help-header">
            <h3>⌨️ 快捷键</h3>
            <button class="close-btn" @click="showHelp = false">×</button>
          </div>
          <div class="help-list">
            <div class="help-item"><kbd>←</kbd> / <kbd>→</kbd><span>上一题 / 下一题</span></div>
            <div class="help-item"><kbd>A</kbd> <kbd>B</kbd> <kbd>C</kbd> <kbd>D</kbd><span>选择对应选项</span></div>
            <div class="help-item"><kbd>Enter</kbd><span>确认 / 下一题</span></div>
            <div class="help-item"><kbd>F</kbd><span>收藏 / 取消收藏</span></div>
            <div class="help-item"><kbd>R</kbd><span>切换 AI 解析</span></div>
            <div class="help-item"><kbd>?</kbd><span>显示 / 隐藏本帮助</span></div>
            <div class="help-item"><kbd>Esc</kbd><span>关闭弹窗</span></div>
          </div>
          <p class="help-tip">提示：快捷键在输入框内不生效</p>
        </div>
      </div>
    </Teleport>

    <div v-if="restoredBanner" class="restored-banner">
      已从上次进度恢复，当前第 {{ current + 1 }} 题
      <button class="close-btn" @click="restoredBanner = false">×</button>
    </div>

    <!-- 模拟考试结果页 -->
    <div v-if="examSubmitted" class="exam-result">
      <h3>模拟考试结束</h3>
      <div class="exam-stats">
        <div class="stat-card">
          <div class="stat-num">{{ examResult.correct }}</div>
          <div class="stat-label">答对</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">{{ examResult.wrong }}</div>
          <div class="stat-label">答错</div>
        </div>
        <div class="stat-card">
          <div class="stat-num">{{ examResult.unanswered }}</div>
          <div class="stat-label">未答</div>
        </div>
        <div class="stat-card highlight">
          <div class="stat-num">{{ examResult.score }}</div>
          <div class="stat-label">得分</div>
        </div>
      </div>
      <p class="hint">总分 100，正确率 {{ examResult.accuracy }}%</p>
      <div class="actions">
        <button @click="restart">再考一次</button>
        <button @click="$router.push(`/wrong/${bankId}`)">查看错题</button>
        <button @click="$router.push('/')">返回题库</button>
      </div>
    </div>

    <div v-else-if="!loaded" class="loading">加载中...</div>
    <div v-else-if="finished && mode !== 'exam'" class="finished">
      <h3>练习完成！</h3>
      <p>共完成 {{ questions.length }} 题</p>
      <button @click="$router.push('/')">返回题库</button>
      <button @click="$router.push(`/wrong/${bankId}`)">查看错题</button>
    </div>
    <div v-else-if="questions.length && current >= 0">
      <QuestionCard
        :key="`${currentQuestion.id}-${reloadKey}`"
        :question="currentQuestion"
        :index="current"
        :auto-next="autoNext"
        :has-prev="current > 0"
        :saved-state="answerStates.get(currentQuestion.id) || null"
        :favorited="favoriteIds.has(currentQuestion.id)"
        :exam-mode="mode === 'exam'"
        @answered="onAnswered"
        @state-change="onStateChange"
        @next="next"
        @prev="prev"
        @toggle-favorite="onToggleFavorite"
        @question-updated="onQuestionUpdated"
      />
      <!-- 题目导航条 -->
      <div v-if="displayQuestions.length > 1" class="question-nav">
        <div class="nav-header">
          <span class="nav-title">题目导航 <span v-if="typeFilter.length" class="filter-hint">（已过滤 {{ typeFilter.length }} 类）</span></span>
          <span class="nav-stats">
            <span class="dot-stat correct">✓ {{ correctCount }}</span>
            <span class="dot-stat wrong">✗ {{ wrongCount }}</span>
            <span class="dot-stat unanswered">○ {{ unansweredCount }}</span>
          </span>
        </div>
        <div class="nav-dots">
          <button
            v-for="(q, i) in displayQuestions"
            :key="q.id"
            class="nav-dot"
            :class="getDotClass(i, q.id)"
            :title="`第 ${i + 1} 题`"
            @click="goToQuestion(i)"
          >{{ i + 1 }}</button>
        </div>
      </div>
    </div>
    <div v-else>暂无题目，请先导入。</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { api, Question } from '../utils/api'
import { toastError } from '../utils/toast'
import { useBankStore } from '../stores/bank'
import QuestionCard, { type QuestionState } from '../components/QuestionCard.vue'

interface SavedProgress {
  mode: string
  order_ids: number[]
  current_id: number
  answer_states: Record<string, QuestionState>
  finished: boolean
  saved_at: string
}

const route = useRoute()
const bankStore = useBankStore()
const bankId = Number(route.params.bankId)
const bankName = computed(() => bankStore.banks.find(b => b.id === bankId)?.name || '')
const questions = ref<Question[]>([])
const order = ref<number[]>([])
const current = ref(0)
const mode = ref('order')
const finished = ref(false)
const autoNext = ref(false)
// 保存每道题的答题状态（按题目 id），切换题目时恢复
const answerStates = ref<Map<number, QuestionState>>(new Map())
// 收藏题目 id 集合
const favoriteIds = ref<Set<number>>(new Set())

const loaded = ref(false)
const restoring = ref(false)
const restoredBanner = ref(false)
const reloadKey = ref(0)

// 搜索相关
const searchQuery = ref('')
const searchResults = ref<Question[] | null>(null)

// 快捷键帮助浮窗
const showHelp = ref(false)

// 题目类型筛选：typeFilter 存储**被排除**的题型（默认全选=空数组）
const typeFilter = ref<string[]>([])
const TYPE_LABELS: Record<string, string> = { single: '单选', multi: '多选', judge: '判断', blank: '填空', qa: '问答' }
const availableTypes = computed(() => {
  const set = new Set<string>()
  for (const q of questions.value) set.add(TYPE_LABELS[q.type] || q.type)
  return Array.from(set)
})
function toggleTypeFilter(t: string) {
  const idx = typeFilter.value.indexOf(t)
  if (idx >= 0) typeFilter.value.splice(idx, 1)
  else typeFilter.value.push(t)
}

// 类型过滤后的题目列表（影响 order）
const displayQuestions = computed(() => {
  if (typeFilter.value.length === 0) return questions.value
  return questions.value.filter(q => {
    const label = TYPE_LABELS[q.type] || q.type
    return !typeFilter.value.includes(label)
  })
})

// 模拟考试相关
const EXAM_DURATION_SECS = 60 * 60 // 60 分钟
const examRemainingSecs = ref(EXAM_DURATION_SECS)
// BUG-004 修复：考试倒计时改用时间戳，避免后台/最小化时 setInterval 被节流导致漂移
let examStartTimeMs: number | null = null
const examSubmitted = ref(false)
const examTimeUp = ref(false)
let examTimerId: number | null = null
const examResult = ref({ correct: 0, wrong: 0, unanswered: 0, score: 0, accuracy: 0 })

const progressKey = `practice_progress_${bankId}`
// 全局最近练习记录（供首页"继续刷题"使用）
const LAST_PRACTICE_KEY = 'last_practice'

const currentQuestion = computed(() => questions.value[order.value[current.value]] || questions.value[0])

function shuffle<T>(arr: T[]): T[] {
  const a = [...arr]
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]]
  }
  return a
}

function formatExamTime(secs: number): string {
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

function startExamTimer() {
  stopExamTimer()
  // BUG-004 修复：记录开始时间戳，每次 tick 用时间差计算剩余秒数
  // 旧实现用 setInterval 自减，后台标签页被节流会严重漂移
  examStartTimeMs = Date.now()
  examRemainingSecs.value = EXAM_DURATION_SECS
  examTimeUp.value = false
  // tick 函数：基于时间戳计算剩余秒数
  const tickExam = () => {
    if (examStartTimeMs === null) return
    const elapsed = Math.floor((Date.now() - examStartTimeMs) / 1000)
    const remaining = EXAM_DURATION_SECS - elapsed
    examRemainingSecs.value = remaining > 0 ? remaining : 0
    if (examRemainingSecs.value <= 0) {
      examRemainingSecs.value = 0
      examTimeUp.value = true
      submitExam()
    }
  }
  // 切回可见时立即校正一次（处理后台节流期间的漂移）
  const onVisible = () => {
    if (document.visibilityState === 'visible') tickExam()
  }
  document.addEventListener('visibilitychange', onVisible)
  ;(window as any).__examVisibilityHandler = onVisible
  examTimerId = window.setInterval(tickExam, 1000)
}
function stopExamTimer() {
  if (examTimerId) {
    window.clearInterval(examTimerId)
    examTimerId = null
  }
  // BUG-004 修复：清理 visibility 监听器
  const handler = (window as any).__examVisibilityHandler
  if (handler) {
    document.removeEventListener('visibilitychange', handler)
    ;(window as any).__examVisibilityHandler = null
  }
  examStartTimeMs = null
}

onMounted(async () => {
  // 全局快捷键
  window.addEventListener('keydown', onKeydown)
  try {
    questions.value = await api.listQuestions(bankId)
    order.value = questions.value.map((_, i) => i)
    // 加载收藏列表
    try {
      const favIds = await api.listFavorites(bankId)
      favoriteIds.value = new Set(favIds)
    } catch (e) {
      console.error('加载收藏列表失败：', e)
    }
    await restoreProgress()
  } catch (e) {
    toastError('加载题目失败：' + (e instanceof Error ? e.message : String(e)))
  } finally {
    loaded.value = true
  }
})

// 从后端恢复上次进度
async function restoreProgress() {
  const saved = await api.getSetting(progressKey)
  if (!saved) return
  let progress: SavedProgress
  try {
    progress = JSON.parse(saved)
  } catch {
    return
  }
  if (!progress || typeof progress.current_id !== 'number') return
  if (!questions.value.length) return

  restoring.value = true

  // 恢复练习模式
  if (progress.mode === 'order' || progress.mode === 'random') {
    mode.value = progress.mode
  }

  // 构建 题目id → 当前索引 的映射（题目列表可能已变化）
  const idToIndex = new Map(questions.value.map((q, i) => [q.id, i]))

  // 恢复题目顺序
  if (progress.order_ids && progress.order_ids.length === questions.value.length) {
    const restoredOrder: number[] = []
    let valid = true
    for (const id of progress.order_ids) {
      const idx = idToIndex.get(id)
      if (idx === undefined) { valid = false; break }
      restoredOrder.push(idx)
    }
    if (valid) {
      order.value = restoredOrder
    } else if (mode.value === 'random') {
      order.value = shuffle(questions.value.map((_, i) => i))
    }
    // 顺序模式下 order.value 已是 [0,1,...,n-1]
  } else if (mode.value === 'random') {
    order.value = shuffle(questions.value.map((_, i) => i))
  }

  // 恢复当前题号（按题目 id 定位，避免题目列表变化导致错位）
  const currentOrderIdx = order.value.findIndex(i => questions.value[i]?.id === progress.current_id)
  current.value = currentOrderIdx >= 0 ? currentOrderIdx : 0

  // 恢复各题答题状态
  if (progress.answer_states) {
    const map = new Map<number, QuestionState>()
    for (const [idStr, state] of Object.entries(progress.answer_states)) {
      const id = Number(idStr)
      if (idToIndex.has(id)) {
        map.set(id, state)
      }
    }
    answerStates.value = map
  }

  if (progress.finished) {
    finished.value = true
  } else if (currentOrderIdx >= 0) {
    restoredBanner.value = true
    setTimeout(() => { restoredBanner.value = false }, 5000)
  }

  await nextTick()
  restoring.value = false
}

// 模式切换：重新生成顺序并回到第一题（恢复阶段跳过）
watch(mode, async (m) => {
  if (restoring.value) return
  examSubmitted.value = false
  if (m === 'random') {
    order.value = shuffle(questions.value.map((_, i) => i))
  } else if (m === 'wrong') {
    // 错题重练：加载错题 ID，按错题顺序生成 order
    try {
      const wrongIds = await api.listWrong(bankId)
      const idToIdx = new Map(questions.value.map((q, i) => [q.id, i]))
      order.value = wrongIds.map(id => idToIdx.get(id)).filter((i): i is number => i !== undefined)
    } catch (e) {
      console.error('加载错题失败：', e)
      order.value = []
    }
  } else if (m === 'exam') {
    // 模拟考试：乱序，启动倒计时
    order.value = shuffle(questions.value.map((_, i) => i))
    answerStates.value = new Map()
    startExamTimer()
  } else {
    order.value = questions.value.map((_, i) => i)
    stopExamTimer()
  }
  current.value = 0
  scheduleSave()
})

// 模拟考试交卷
function submitExam() {
  stopExamTimer()
  let correct = 0, wrong = 0, unanswered = 0
  for (const idx of order.value) {
    const q = questions.value[idx]
    if (!q) continue
    const state = answerStates.value.get(q.id)
    if (!state || !state.submitted) {
      unanswered++
    } else if (state.isCorrect) {
      correct++
    } else {
      wrong++
    }
  }
  const total = order.value.length || 1
  const accuracy = Math.round((correct / total) * 100)
  const score = Math.round((correct / total) * 100)
  examResult.value = { correct, wrong, unanswered, score, accuracy }
  examSubmitted.value = true
}

// 防抖保存进度
let saveTimer: ReturnType<typeof setTimeout> | null = null
function scheduleSave() {
  if (restoring.value) return
  if (!loaded.value) return
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(saveProgress, 500)
}

async function saveProgress() {
  if (!questions.value.length || !order.value.length) return
  // 完成练习后清除进度，下次从头开始
  if (finished.value) {
    try {
      await api.setSetting(progressKey, '')
    } catch (e) {
      console.error('清除进度失败：', e)
    }
    return
  }
  const cur = order.value[current.value]
  if (cur === undefined) return
  const progress: SavedProgress = {
    mode: mode.value,
    order_ids: order.value
      .map(i => questions.value[i]?.id)
      .filter((id): id is number => id !== undefined),
    current_id: questions.value[cur].id,
    answer_states: Object.fromEntries(answerStates.value.entries()),
    finished: false,
    saved_at: new Date().toISOString(),
  }
  try {
    await api.setSetting(progressKey, JSON.stringify(progress))
    // 同步更新全局最近练习记录（首页"继续刷题"卡片使用）
    const lastPractice = {
      bank_id: bankId,
      bank_name: bankName.value,
      position: current.value + 1,
      total: questions.value.length,
      saved_at: progress.saved_at,
    }
    await api.setSetting(LAST_PRACTICE_KEY, JSON.stringify(lastPractice))
  } catch (e) {
    console.error('保存进度失败：', e)
  }
}

// 切换当前题目收藏状态
async function onToggleFavorite() {
  const q = currentQuestion.value
  if (!q) return
  try {
    const nowFav = await api.toggleFavorite(bankId, q.id)
    const next = new Set(favoriteIds.value)
    if (nowFav) next.add(q.id)
    else next.delete(q.id)
    favoriteIds.value = next
  } catch (e) {
    console.error('切换收藏失败：', e)
  }
}

// 题目编辑保存后，更新本地 questions 数组并强制重渲染当前题
function onQuestionUpdated(updated: Question) {
  const idx = questions.value.findIndex(q => q.id === updated.id)
  if (idx >= 0) {
    questions.value[idx] = updated
  }
  reloadKey.value++  // 强制 QuestionCard 重新挂载，加载新数据
}

watch([current, finished], scheduleSave)
watch(order, scheduleSave, { deep: true })

// 组件卸载前立即保存一次，避免导航离开时丢失最后一次进度
// 全局快捷键处理
function onKeydown(e: KeyboardEvent) {
  // 在输入框/textarea 中不响应
  const t = e.target as HTMLElement
  if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return
  if (e.key === '?' || (e.shiftKey && e.key === '/')) {
    e.preventDefault()
    showHelp.value = !showHelp.value
  } else if (e.key === 'Escape') {
    showHelp.value = false
  }
  // BUG-005 修复：移除 F 键分支，由 QuestionCard 单独处理避免双触发
  // 旧实现：父组件 onKeydown + 子组件 handleKeydown 都监听 window keydown，
  // 按一次 F 收藏被切换两次，净效果为未变化
}

onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown)
  stopExamTimer()
  if (saveTimer) {
    clearTimeout(saveTimer)
    saveTimer = null
    void saveProgress()
  }
})

async function onAnswered(payload: { correct: boolean; answer: string; duration_ms: number | null }) {
  const q = currentQuestion.value
  try {
    await api.recordPractice({ bank_id: bankId, question_id: q.id, user_answer: payload.answer, is_correct: payload.correct, duration_ms: payload.duration_ms })
    // 累计每日统计（用于热力图）
    await bumpDailyRecord(payload.correct)
  } catch (e) {
    console.error('记录练习失败：', e)
  }
}

// 累加今日刷题记录到 settings.daily_records
// BUG-008 修复：加前端互斥锁，避免连续答题时 read-modify-write 竞态导致统计丢失
// 旧实现：两次并发 bumpDailyRecord 都读到同一份 records，各自 +1 后写回，后写覆盖先写
let bumpDailyRecordChain: Promise<void> = Promise.resolve()
function bumpDailyRecord(correct: boolean): Promise<void> {
  // 串行化：将每次调用接到 chain 末尾，保证不并发
  const run = async () => {
    try {
      const raw = await api.getSetting('daily_records')
      const records: { date: string; total: number; correct: number }[] = raw ? JSON.parse(raw) : []
      const d = new Date()
      const today = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
      const idx = records.findIndex(r => r.date === today)
      if (idx >= 0) {
        records[idx].total++
        if (correct) records[idx].correct++
      } else {
        records.push({ date: today, total: 1, correct: correct ? 1 : 0 })
      }
      // 只保留最近 400 天
      const sorted = records.sort((a, b) => a.date.localeCompare(b.date))
      const trimmed = sorted.slice(-400)
      await api.setSetting('daily_records', JSON.stringify(trimmed))
    } catch (e) {
      console.error('更新每日统计失败：', e)
    }
  }
  bumpDailyRecordChain = bumpDailyRecordChain.then(run)
  return bumpDailyRecordChain
}

// P1-10: 搜索题目，跳转到第一个匹配
async function doSearch() {
  const q = searchQuery.value.trim()
  if (!q) {
    clearSearch()
    return
  }
  try {
    const results = await api.searchQuestions(bankId, q, 50)
    searchResults.value = results
    if (results.length > 0) {
      // 跳到第一个匹配的题目
      const firstId = results[0].id
      const idx = order.value.findIndex(i => questions.value[i]?.id === firstId)
      if (idx >= 0) {
        current.value = idx
      }
    }
  } catch (e) {
    toastError('搜索失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

function clearSearch() {
  searchResults.value = null
  searchQuery.value = ''
}

// 高亮关键词：转义 HTML 后用 <mark> 包裹
function highlightText(text: string, keyword: string): string {
  if (!text) return ''
  const safe = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  if (!keyword.trim()) return safe
  const escapedKw = keyword.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return safe.replace(new RegExp(escapedKw, 'gi'), m => `<mark>${m}</mark>`)
}

// 跳转到指定题目
function jumpToQuestion(qid: number) {
  const orderIdx = order.value.findIndex(i => questions.value[i]?.id === qid)
  if (orderIdx >= 0) {
    current.value = orderIdx
    nextTick(() => {
      const el = document.querySelector('.qcard')
      el?.scrollIntoView({ behavior: 'smooth', block: 'start' })
    })
  }
}

// QuestionCard 状态变化时保存到 Map（切换题目/返回上一题时仍可恢复）
function onStateChange(state: QuestionState) {
  const q = currentQuestion.value
  answerStates.value.set(q.id, state)
  scheduleSave()
}

function next() {
  if (current.value < order.value.length - 1) {
    current.value++
  } else {
    finished.value = true
  }
}

function prev() {
  if (current.value > 0) {
    current.value--
  }
}

function goToQuestion(displayIndex: number) {
  // displayIndex 是 displayQuestions 的下标，需要找到对应 qid，再找到 order 中的 current 位置
  const target = displayQuestions.value[displayIndex]
  if (!target) return
  const orderIdx = order.value.findIndex(i => questions.value[i]?.id === target.id)
  if (orderIdx >= 0) {
    current.value = orderIdx
  }
}

// 导航点状态：current/submitted/correct/wrong/fav
function getDotClass(_listIndex: number, qid: number): string {
  const classes: string[] = []
  // current 通过 qid 比较（因为 listIndex 是 displayQuestions 下标，current 是 order 下标）
  const currentQid = currentQuestion.value?.id
  if (qid === currentQid) classes.push('current')
  const state = answerStates.value.get(qid)
  if (state?.submitted) {
    if (state.isCorrect === true) classes.push('correct')
    else if (state.isCorrect === false) classes.push('wrong')
    else classes.push('submitted')
  }
  if (favoriteIds.value.has(qid)) classes.push('fav')
  return classes.join(' ')
}

const correctCount = computed(() => {
  let c = 0
  for (const s of answerStates.value.values()) if (s.submitted && s.isCorrect === true) c++
  return c
})
const wrongCount = computed(() => {
  let c = 0
  for (const s of answerStates.value.values()) if (s.submitted && s.isCorrect === false) c++
  return c
})
const unansweredCount = computed(() => {
  const submitted = correctCount.value + wrongCount.value
  return Math.max(0, questions.value.length - submitted)
})

// 从头开始：清除进度并重置
async function restart() {
  if (mode.value === 'exam' && !examSubmitted.value && !confirm('确定要重新开始考试吗？当前答题进度将丢失。')) return
  if (mode.value !== 'exam' && !confirm('确定要从头开始吗？当前进度将被清除。')) return
  restoring.value = true
  stopExamTimer()
  examSubmitted.value = false
  examTimeUp.value = false
  current.value = 0
  mode.value = 'order'
  order.value = questions.value.map((_, i) => i)
  answerStates.value = new Map()
  finished.value = false
  restoredBanner.value = false
  reloadKey.value++
  await nextTick()
  restoring.value = false
  await saveProgress()
}
</script>

<style scoped>
/* topbar 合并两排布局 */
.topbar { display: flex; gap: 12px; align-items: center; margin-bottom: 10px; flex-wrap: wrap; padding: 10px 14px; background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-md); }
.main-bar { padding: 12px 16px; }
.main-left { display: flex; align-items: center; gap: 10px; }
.main-left .bank-name { font-weight: 600; color: var(--color-text); font-size: 15px; }
.main-progress { flex: 1; display: flex; align-items: center; gap: 10px; min-width: 160px; }
.main-progress .progress-text { color: var(--color-text-secondary); font-size: 13px; font-family: monospace; white-space: nowrap; }
.progress-track { flex: 1; height: 6px; background: var(--color-border-light); border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: linear-gradient(90deg, var(--color-primary), var(--color-primary-dark)); border-radius: 3px; transition: width 0.3s; }
.main-right { display: flex; align-items: center; gap: 8px; }
.tool-bar { font-size: 13px; padding: 8px 14px; }
.info-bar { font-size: 15px; }
.info-bar .bank-name { font-weight: 600; color: var(--color-text); }
.info-bar .progress { color: var(--color-text-secondary); padding: 2px 10px; background: var(--color-border-light); border-radius: var(--radius-sm); font-size: 13px; font-family: monospace; }
.control-bar { padding-bottom: 8px; border-bottom: 1px solid var(--color-border-light); }
.action-bar { justify-content: flex-end; margin-top: 4px; }

.mode-select { padding: 5px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); font-size: 13px; cursor: pointer; }
.help-btn { width: 28px; height: 28px; border-radius: 50%; border: 1px solid var(--color-border); background: var(--color-card); cursor: pointer; font-size: 14px; font-weight: 700; color: var(--color-text-secondary); }
.help-btn:hover { background: var(--color-primary-light); color: var(--color-primary); border-color: var(--color-primary); }

.type-filter { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
.filter-label { font-size: 12px; color: var(--color-text-tertiary); margin-right: 2px; }
.type-chip-btn { padding: 3px 10px; border: 1px solid var(--color-border); border-radius: 12px; background: var(--color-bg); color: var(--color-text-tertiary); font-size: 12px; cursor: pointer; transition: all 0.12s; opacity: 0.5; }
.type-chip-btn.active { background: var(--color-primary-light); color: var(--color-primary); border-color: var(--color-primary); opacity: 1; }
.type-chip-btn:hover { transform: translateY(-1px); }
.filter-hint { font-size: 11px; color: var(--color-warning); margin-left: 6px; }

.help-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.45); display: flex; align-items: center; justify-content: center; z-index: 200; animation: helpFadeIn 0.15s; }
.help-modal { background: var(--color-card); border-radius: var(--radius-lg); padding: 24px; min-width: 360px; max-width: 90vw; color: var(--color-text); box-shadow: 0 16px 48px rgba(0,0,0,0.25); }
.help-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.help-header h3 { margin: 0; }
.help-list { display: flex; flex-direction: column; gap: 10px; }
.help-item { display: flex; align-items: center; gap: 8px; font-size: 14px; }
.help-item span { margin-left: auto; color: var(--color-text-secondary); }
.help-item kbd { display: inline-block; padding: 2px 8px; background: var(--color-bg); border: 1px solid var(--color-border); border-bottom-width: 2px; border-radius: 4px; font-family: ui-monospace, Consolas, monospace; font-size: 12px; color: var(--color-text); }
.help-tip { margin-top: 16px; padding-top: 12px; border-top: 1px solid var(--color-border-light); color: var(--color-text-tertiary); font-size: 12px; }

@keyframes helpFadeIn { from { opacity: 0; } to { opacity: 1; } }

/* 题目导航条 */
.question-nav { margin-top: 18px; padding: 14px 16px; background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-md); }
.nav-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.nav-title { font-size: 13px; font-weight: 600; color: var(--color-text); }
.nav-stats { display: flex; gap: 10px; font-size: 12px; }
.dot-stat { padding: 1px 8px; border-radius: 10px; }
.dot-stat.correct { background: #dcfce7; color: #15803d; }
.dot-stat.wrong { background: #fee2e2; color: #b91c1c; }
.dot-stat.unanswered { background: var(--color-border-light); color: var(--color-text-secondary); }
.nav-dots { display: flex; flex-wrap: wrap; gap: 4px; max-height: 180px; overflow-y: auto; padding: 2px; }
.nav-dot { min-width: 28px; height: 28px; padding: 0 4px; border: 1px solid var(--color-border); border-radius: 6px; background: var(--color-card); color: var(--color-text-secondary); font-size: 11px; cursor: pointer; transition: all 0.12s; display: inline-flex; align-items: center; justify-content: center; font-weight: 500; }
.nav-dot:hover { transform: translateY(-1px); box-shadow: 0 2px 6px rgba(0,0,0,0.1); border-color: var(--color-primary); color: var(--color-primary); }
.nav-dot.current { background: var(--color-primary); color: #fff; border-color: var(--color-primary); box-shadow: 0 0 0 2px var(--color-primary-light); }
.nav-dot.correct { background: #dcfce7; color: #15803d; border-color: #bbf7d0; }
.nav-dot.wrong { background: #fee2e2; color: #b91c1c; border-color: #fecaca; }
.nav-dot.submitted { background: #fef3c7; color: #b45309; border-color: #fde68a; }
.nav-dot.fav::after { content: "★"; color: #f59e0b; font-size: 8px; margin-left: 2px; }
.search-box { display: flex; gap: 4px; align-items: center; }
.search-box input { padding: 5px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); width: 200px; background: var(--color-card); color: var(--color-text); font-size: 13px; }
.search-btn { padding: 5px 12px; border: 1px solid var(--color-primary); background: var(--color-primary); color: #fff; border-radius: var(--radius-md); cursor: pointer; font-size: 12px; }
.clear-search-btn { padding: 5px 8px; border: 1px solid var(--color-border); background: var(--color-card); color: var(--color-text-secondary); border-radius: var(--radius-md); cursor: pointer; font-size: 12px; }
.auto-next-toggle { display: flex; align-items: center; gap: 4px; font-size: 13px; cursor: pointer; color: var(--color-text-secondary); }
.auto-next-toggle input { cursor: pointer; }

.restart-btn { padding: 5px 14px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); cursor: pointer; font-size: 13px; color: var(--color-text); }
.restart-btn:hover { background: var(--color-border-light); }
.submit-exam-btn { padding: 5px 14px; border: 1px solid var(--color-danger); border-radius: var(--radius-md); background: var(--color-danger); color: #fff; cursor: pointer; font-size: 13px; }
.submit-exam-btn:hover { opacity: 0.9; }

.exam-timer { font-family: monospace; font-size: 14px; font-weight: 600; padding: 4px 12px; background: var(--color-info-light); color: var(--color-info); border-radius: var(--radius-md); }
.exam-timer.time-up { background: var(--color-danger-light); color: var(--color-danger); animation: pulse 1s infinite; }
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.5; } }

.search-result-banner { background: var(--color-info-light); color: var(--color-info); padding: 8px 16px; border-radius: var(--radius-md); margin-bottom: 12px; display: flex; justify-content: space-between; align-items: center; font-size: 13px; }
.search-result-banner b { color: var(--color-primary); font-weight: 600; }
.search-result-list { max-height: 360px; overflow-y: auto; background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-md); margin-bottom: 16px; }
.search-result-item { display: flex; align-items: flex-start; gap: 8px; padding: 10px 14px; border-bottom: 1px solid var(--color-border-light); cursor: pointer; transition: background 0.12s; }
.search-result-item:last-child { border-bottom: none; }
.search-result-item:hover { background: var(--color-primary-light); }
.search-result-idx { color: var(--color-text-tertiary); font-size: 13px; flex-shrink: 0; }
.search-result-text { flex: 1; font-size: 13px; line-height: 1.5; color: var(--color-text); display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
.search-result-text :deep(mark) { background: #fef3c7; color: #b45309; padding: 0 2px; border-radius: 2px; font-weight: 600; }
.search-result-types { display: flex; gap: 4px; flex-shrink: 0; }
.type-chip { padding: 1px 6px; background: var(--color-border-light); color: var(--color-text-secondary); border-radius: 3px; font-size: 11px; }
.search-result-overflow { padding: 8px 14px; font-size: 12px; color: var(--color-text-tertiary); text-align: center; background: var(--color-bg); border-top: 1px solid var(--color-border-light); }
.restored-banner { background: var(--color-success-light); border: 1px solid var(--color-success); color: var(--color-success); padding: 8px 16px; border-radius: var(--radius-md); margin-bottom: 16px; display: flex; justify-content: space-between; align-items: center; font-size: 14px; }
.restored-banner .close-btn { background: none; border: none; font-size: 18px; cursor: pointer; color: var(--color-success); padding: 0 4px; line-height: 1; }
.loading { text-align: center; padding: 48px; color: var(--color-text-tertiary); }
.finished { text-align: center; padding: 48px; }
.finished button { margin: 8px; padding: 8px 16px; }
.exam-result { text-align: center; padding: 32px; }
.exam-result h3 { margin-bottom: 24px; }
.exam-stats { display: flex; gap: 16px; justify-content: center; flex-wrap: wrap; margin-bottom: 16px; }
.stat-card { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 16px 24px; min-width: 100px; }
.stat-card.highlight { background: var(--color-primary); color: #fff; border-color: var(--color-primary); }
.stat-num { font-size: 28px; font-weight: 600; }
.stat-label { font-size: 13px; opacity: 0.8; margin-top: 4px; }
.exam-result .hint { color: var(--color-text-secondary); margin: 16px 0; }
.exam-result .actions { margin-top: 24px; }
.exam-result .actions button { margin: 8px; padding: 8px 16px; }
</style>
