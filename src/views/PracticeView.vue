<template>
  <div class="practice">
    <!-- 信息区：题库名 + 进度 + 考试倒计时 -->
    <div class="topbar info-bar">
      <span class="bank-name">{{ bankName }}</span>
      <span class="progress">{{ current + 1 }} / {{ questions.length }}</span>
      <span v-if="mode === 'exam'" class="exam-timer" :class="{ 'time-up': examTimeUp }">
        ⏱ 剩余 {{ formatExamTime(examRemainingSecs) }}
      </span>
    </div>

    <!-- 控制区：模式 / 搜索 / 自动下一题 -->
    <div class="topbar control-bar">
      <select v-model="mode" class="mode-select">
        <option value="order">顺序练习</option>
        <option value="random">随机练习</option>
        <option value="wrong">错题重练</option>
        <option value="exam">模拟考试</option>
      </select>
      <div class="search-box">
        <input v-model="searchQuery" placeholder="搜索题目..." @keyup.enter="doSearch" />
        <button v-if="searchQuery" class="search-btn" @click="doSearch">搜索</button>
        <button v-if="searchResults" class="clear-search-btn" @click="clearSearch">✕</button>
      </div>
      <label v-if="mode !== 'exam'" class="auto-next-toggle">
        <input type="checkbox" v-model="autoNext" />
        答对自动下一题
      </label>
    </div>

    <!-- 操作区：从头开始 / 交卷 -->
    <div class="topbar action-bar">
      <button class="restart-btn" @click="restart">从头开始</button>
      <button v-if="mode === 'exam' && !examSubmitted" class="submit-exam-btn" @click="submitExam">交卷</button>
    </div>

    <div v-if="searchResults" class="search-result-banner">
      搜索到 {{ searchResults.length }} 道相关题目
      <button class="close-btn" @click="clearSearch">✕</button>
    </div>

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
    </div>
    <div v-else>暂无题目，请先导入。</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { useRoute } from 'vue-router'
import { api, Question } from '../utils/api'
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

// 模拟考试相关
const EXAM_DURATION_SECS = 60 * 60 // 60 分钟
const examRemainingSecs = ref(EXAM_DURATION_SECS)
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
  examRemainingSecs.value = EXAM_DURATION_SECS
  examTimeUp.value = false
  examTimerId = window.setInterval(() => {
    examRemainingSecs.value--
    if (examRemainingSecs.value <= 0) {
      examRemainingSecs.value = 0
      examTimeUp.value = true
      submitExam()
    }
  }, 1000)
}
function stopExamTimer() {
  if (examTimerId) {
    window.clearInterval(examTimerId)
    examTimerId = null
  }
}

onMounted(async () => {
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
    alert('加载题目失败：' + (e instanceof Error ? e.message : String(e)))
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
onBeforeUnmount(() => {
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
  } catch (e) {
    console.error('记录练习失败：', e)
  }
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
    alert('搜索失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

function clearSearch() {
  searchQuery.value = ''
  searchResults.value = null
}

// QuestionCard 状态变化时保存到 Map（切换题目/返回上一题时仍可恢复）
function onStateChange(state: QuestionState) {
  const q = currentQuestion.value
  answerStates.value.set(q.id, state)
  scheduleSave()
}

function next() {
  if (current.value < questions.value.length - 1) {
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
/* topbar 三组分区布局 */
.topbar { display: flex; gap: 12px; align-items: center; margin-bottom: 8px; flex-wrap: wrap; }
.info-bar { font-size: 15px; }
.info-bar .bank-name { font-weight: 600; color: var(--color-text); }
.info-bar .progress { color: var(--color-text-secondary); padding: 2px 10px; background: var(--color-border-light); border-radius: var(--radius-sm); font-size: 13px; font-family: monospace; }
.control-bar { padding-bottom: 8px; border-bottom: 1px solid var(--color-border-light); }
.action-bar { justify-content: flex-end; margin-top: 4px; }

.mode-select { padding: 5px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); font-size: 13px; cursor: pointer; }
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

.search-result-banner { background: var(--color-info-light); color: var(--color-info); padding: 8px 16px; border-radius: var(--radius-md); margin-bottom: 16px; display: flex; justify-content: space-between; align-items: center; font-size: 13px; }
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
