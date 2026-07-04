<template>
  <div class="qcard">
    <div class="stem">
      <span class="idx">{{ index + 1 }}.</span>
      <span class="type-tag">{{ typeLabel }}</span>
      <span v-if="favorited" class="fav-mark" title="已收藏">★</span>
      <span v-if="elapsedSecs !== null" class="timer" title="本题用时">{{ formatTime(elapsedSecs) }}</span>
      {{ question.stem }}
    </div>

    <!-- 选择题 -->
    <div v-if="isChoice" class="options">
      <button
        v-for="(opt, i) in options"
        :key="i"
        class="option"
        :class="optionClass(i)"
        :disabled="submitted"
        @click="toggle(i)"
      >
        <span class="letter">{{ letter(i) }}</span> {{ opt }}
      </button>
    </div>

    <!-- 判断题 -->
    <div v-else-if="question.type === 'judge'" class="options">
      <button class="option" :class="judgeClass(true)" :disabled="submitted" @click="answerJudge(true)">√ 正确</button>
      <button class="option" :class="judgeClass(false)" :disabled="submitted" @click="answerJudge(false)">× 错误</button>
    </div>

    <!-- 填空/问答 -->
    <div v-else class="blank">
      <textarea v-model="blankAnswer" :disabled="submitted" placeholder="输入你的答案"></textarea>
    </div>

    <div class="actions">
      <button v-if="hasPrev" @click="$emit('prev')">上一题</button>
      <button v-if="!submitted" @click="submit">确认</button>
      <button v-if="submitted && (!isSelfEval || selfEvalDone)" @click="$emit('next')">下一题</button>
      <button v-if="submitted && !isChoice && question.type !== 'judge'" @click="selfEval(true)">答对</button>
      <button v-if="submitted && !isChoice && question.type !== 'judge'" @click="selfEval(false)">答错</button>
      <button class="fav-toggle" :class="{ active: favorited }" @click="$emit('toggle-favorite')">{{ favorited ? '★ 已收藏' : '☆ 收藏' }}</button>
      <button class="edit-btn" @click="showEdit = true">✎ 编辑</button>
      <button class="ai-btn" :disabled="analyzing" @click="analyze">{{ analyzing ? '解析中…' : 'AI 解析' }}</button>
    </div>

    <QuestionEditDialog :visible="showEdit" :question="question" @close="showEdit = false" @saved="onQuestionSaved" />

    <div class="hint" v-if="!submitted">快捷键：{{ keyHint }}</div>

    <div v-if="submitted" class="feedback" :class="{ correct: isCorrect, exam: examMode }">
      <template v-if="examMode">
        <p>✓ 已作答（考试模式不立即显示对错）</p>
      </template>
      <template v-else>
        <p v-if="isSelfEval && !selfEvalDone">请对照参考答案自评</p>
        <p v-else-if="!question.answer && question.type === 'judge'">⚠ 参考答案缺失，无法判定对错</p>
        <p v-else>{{ isCorrect ? '✓ 回答正确' : '✗ 回答错误' }}</p>
        <p>正确答案：{{ displayAnswer }}</p>
        <div v-if="question.analysis" class="analysis">
          <strong>解析：</strong>{{ question.analysis }}
        </div>
      </template>
    </div>

    <div v-if="aiAnalysis" class="ai-analysis">
      <strong>AI 解析：</strong>
      <pre>{{ aiAnalysis }}</pre>
    </div>
    <div v-if="aiError" class="ai-error">{{ aiError }}</div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { api, Question } from '../utils/api'
import QuestionEditDialog from './QuestionEditDialog.vue'

export interface QuestionState {
  selected: number[]
  blankAnswer: string
  submitted: boolean
  isCorrect: boolean
  selfEvalDone: boolean
  judgeSelected: boolean | null
  elapsedSecs: number | null
}

const props = defineProps<{
  question: Question
  index: number
  autoNext?: boolean
  hasPrev?: boolean
  savedState?: QuestionState | null
  favorited?: boolean
  examMode?: boolean
}>()
const emit = defineEmits<{
  (e: 'answered', payload: { correct: boolean; answer: string; duration_ms: number | null }): void
  (e: 'state-change', state: QuestionState): void
  (e: 'next'): void
  (e: 'prev'): void
  (e: 'toggle-favorite'): void
  (e: 'question-updated', q: Question): void
}>()

// 从保存的状态恢复（返回上一题时能看到之前的答案）
const saved = props.savedState
const selected = ref<number[]>(saved?.selected ? [...saved.selected] : [])
const blankAnswer = ref(saved?.blankAnswer ?? '')
const submitted = ref(saved?.submitted ?? false)
const isCorrect = ref(saved?.isCorrect ?? false)
const selfEvalDone = ref(saved?.selfEvalDone ?? false)
const judgeSelected = ref<boolean | null>(saved?.judgeSelected ?? null)

// 单题计时（spec §5.2）
const startTime = ref<number | null>(null)
const elapsedSecs = ref<number | null>(saved?.elapsedSecs ?? null)
let timerId: number | null = null

function startTimer() {
  if (timerId) return
  startTime.value = Date.now()
  const baseSecs = elapsedSecs.value ?? 0
  timerId = window.setInterval(() => {
    if (startTime.value) {
      elapsedSecs.value = baseSecs + Math.floor((Date.now() - startTime.value) / 1000)
    }
  }, 1000)
}
function stopTimer() {
  if (timerId) {
    window.clearInterval(timerId)
    timerId = null
  }
}
function formatTime(secs: number): string {
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
}

onMounted(() => {
  // 已提交的题不重启计时
  if (!submitted.value) startTimer()
})
onBeforeUnmount(() => stopTimer())

// AI 解析相关
const analyzing = ref(false)
const aiAnalysis = ref('')
const aiError = ref('')

// 题目编辑
const showEdit = ref(false)
function onQuestionSaved(updated: Question) {
  showEdit.value = false
  // 通知父组件题目已更新（父组件需要更新 questions 数组）
  emit('question-updated', updated)
}

const options = computed<string[]>(() => {
  if (!props.question.options) return []
  try {
    return JSON.parse(props.question.options)
  } catch {
    return []
  }
})
const isChoice = computed(() => ['single', 'multi'].includes(props.question.type))
const isSelfEval = computed(() => !isChoice.value && props.question.type !== 'judge')
const typeLabel = computed(() => ({ single: '单选', multi: '多选', judge: '判断', blank: '填空', qa: '问答' }[props.question.type] || props.question.type))
const displayAnswer = computed(() => {
  if (!props.question.answer) return '（未识别到答案）'
  if (props.question.type === 'judge') return props.question.answer === 'true' ? '正确' : '错误'
  return props.question.answer
})
const keyHint = computed(() => {
  if (isChoice.value) return '1-4 选选项，Enter 确认，← → 翻页，F 收藏'
  if (props.question.type === 'judge') return '1 正确 / 2 错误，← → 翻页，F 收藏'
  return '输入答案后 Enter 确认，← → 翻页，F 收藏'
})

function emitState() {
  emit('state-change', {
    selected: [...selected.value],
    blankAnswer: blankAnswer.value,
    submitted: submitted.value,
    isCorrect: isCorrect.value,
    selfEvalDone: selfEvalDone.value,
    judgeSelected: judgeSelected.value,
    elapsedSecs: elapsedSecs.value,
  })
}

// 监听状态变化，及时同步给父组件保存
watch([selected, blankAnswer, submitted, isCorrect, selfEvalDone, judgeSelected, elapsedSecs], emitState, { deep: true })

function letter(i: number) { return String.fromCharCode(65 + i) }
function toggle(i: number) {
  if (props.question.type === 'single') { selected.value = [i] }
  else {
    const idx = selected.value.indexOf(i)
    if (idx >= 0) selected.value.splice(idx, 1)
    else selected.value.push(i)
  }
}
function optionClass(i: number) {
  if (!submitted.value) return { selected: selected.value.includes(i) }
  const correctLetters = parseAnswerLetters()
  const isAns = correctLetters.includes(i)
  const isPicked = selected.value.includes(i)
  return { correct: isAns, wrong: isPicked && !isAns }
}
function judgeClass(val: boolean) {
  if (!submitted.value) return { selected: judgeSelected.value === val }
  // 答案缺失时只高亮用户选择，不标绿/红
  if (!props.question.answer) return { selected: judgeSelected.value === val }
  const ans = props.question.answer === 'true'
  const isCorrectOption = (val === ans)
  // 答对时：只标绿正确选项，不标红错误选项；答错时：错项标红，正确项标绿
  if (isCorrect.value) {
    return { correct: isCorrectOption }  // 答对了，只显示正确答案为绿色，错误选项不高亮
  }
  // 答错了：用户选的标红，正确答案标绿
  return { correct: isCorrectOption, wrong: !isCorrectOption && judgeSelected.value === val }
}
function parseAnswerLetters(): number[] {
  if (!props.question.answer) return []
  try {
    const arr = JSON.parse(props.question.answer) as string[]
    return arr.map(s => s.charCodeAt(0) - 65)
  } catch {
    return props.question.answer.split('').filter(c => /[A-D]/i.test(c)).map(c => c.toUpperCase().charCodeAt(0) - 65)
  }
}
function getDurationMs(): number | null {
  if (elapsedSecs.value === null) return null
  return elapsedSecs.value * 1000
}
function answerJudge(val: boolean) {
  judgeSelected.value = val
  submitted.value = true
  // 答案缺失时无法判定对错，记为错误但不影响错题本判定逻辑
  isCorrect.value = props.question.answer ? (val === (props.question.answer === 'true')) : false
  stopTimer()
  emit('answered', { correct: isCorrect.value, answer: String(val), duration_ms: getDurationMs() })
  maybeAutoNext()
}
function submit() {
  submitted.value = true
  stopTimer()
  if (isChoice.value) {
    const picked = [...selected.value].sort().map(i => String.fromCharCode(65 + i))
    const correct = parseAnswerLetters().sort().map(i => String.fromCharCode(65 + i))
    isCorrect.value = JSON.stringify(picked) === JSON.stringify(correct)
    emit('answered', { correct: isCorrect.value, answer: JSON.stringify(picked), duration_ms: getDurationMs() })
    maybeAutoNext()
  } else if (props.question.type === 'judge') {
    // 判断题通过 answerJudge 处理，不会走到这里
  } else {
    // 填空/问答：展示参考答案，等自评，不设置 isCorrect
    isCorrect.value = false
  }
}
function selfEval(correct: boolean) {
  isCorrect.value = correct
  selfEvalDone.value = true
  emit('answered', { correct, answer: blankAnswer.value, duration_ms: getDurationMs() })
  maybeAutoNext()
}
function maybeAutoNext() {
  // 答对且开启自动切换时，1.5秒后自动跳转下一题；答错不自动跳
  // 考试模式下不自动跳，让用户自己控制节奏
  if (props.examMode) return
  if (props.autoNext && isCorrect.value) {
    setTimeout(() => emit('next'), 1500)
  }
}

// P1-9: 快捷键支持
function handleKeydown(e: KeyboardEvent) {
  // 忽略输入框中的按键（避免影响填空答题）
  const target = e.target as HTMLElement
  if (target && (target.tagName === 'TEXTAREA' || target.tagName === 'INPUT')) {
    if (e.key === 'Enter' && !e.shiftKey && !submitted.value) {
      e.preventDefault()
      submit()
    }
    return
  }
  // 已提交时：← → 翻页，Enter 下一题，F 收藏
  if (submitted.value && (!isSelfEval.value || selfEvalDone.value)) {
    if (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      emit('next')
      return
    }
  }
  // 考试模式：提交后也可以 Enter 进入下一题
  if (submitted.value && props.examMode) {
    if (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      emit('next')
      return
    }
  }
  if (e.key === 'ArrowLeft' && props.hasPrev) {
    e.preventDefault()
    emit('prev')
    return
  }
  if (e.key === 'f' || e.key === 'F') {
    e.preventDefault()
    emit('toggle-favorite')
    return
  }
  // 未提交时：1-4 选 ABCD（选择题），1/2 选正确/错误（判断题），Enter 确认
  if (!submitted.value) {
    if (isChoice.value) {
      const n = parseInt(e.key, 10)
      if (n >= 1 && n <= options.value.length) {
        e.preventDefault()
        toggle(n - 1)
        return
      }
    } else if (props.question.type === 'judge') {
      if (e.key === '1') { e.preventDefault(); answerJudge(true); return }
      if (e.key === '2') { e.preventDefault(); answerJudge(false); return }
    }
    if (e.key === 'Enter' && isChoice.value) {
      e.preventDefault()
      submit()
    }
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})
onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeydown)
})

async function analyze() {
  if (analyzing.value) return
  analyzing.value = true
  aiError.value = ''
  aiAnalysis.value = ''
  try {
    aiAnalysis.value = await api.analyzeQuestion(props.question)
  } catch (e) {
    aiError.value = '解析失败：' + (e instanceof Error ? e.message : String(e))
  } finally {
    analyzing.value = false
  }
}
</script>

<style scoped>
.qcard { background: var(--color-card); border-radius: var(--radius-lg); padding: 24px; border: 1px solid var(--color-border-light); }
.stem { font-size: 16px; line-height: 1.6; margin-bottom: 16px; position: relative; }
.idx { font-weight: bold; margin-right: 8px; }
.type-tag { background: var(--color-border-light); padding: 2px 8px; border-radius: var(--radius-sm); font-size: 12px; margin-right: 8px; }
.fav-mark { color: var(--color-warning); margin-right: 8px; font-size: 14px; }
.timer { color: var(--color-text-tertiary); font-size: 12px; margin-right: 8px; font-family: monospace; background: var(--color-border-light); padding: 2px 6px; border-radius: var(--radius-sm); }
.fav-toggle { padding: 6px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); cursor: pointer; color: var(--color-text-secondary); }
.edit-btn { padding: 6px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); cursor: pointer; color: var(--color-text-secondary); }
.edit-btn:hover { background: var(--color-border-light); color: var(--color-primary); }
.fav-toggle:hover { background: var(--color-warning-light); border-color: var(--color-warning); }
.fav-toggle.active { color: var(--color-warning); border-color: var(--color-warning); background: var(--color-warning-light); }
.options { display: flex; flex-direction: column; gap: 8px; }
.option { text-align: left; padding: 12px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); cursor: pointer; color: var(--color-text); }
.option.selected { border-color: var(--color-primary); background: var(--color-primary-light); }
.option.correct { border-color: var(--color-success); background: var(--color-success-light); }
.option.wrong { border-color: var(--color-danger); background: var(--color-danger-light); }
.letter { font-weight: bold; margin-right: 8px; }
.actions { margin-top: 16px; display: flex; gap: 8px; flex-wrap: wrap; }
.hint { margin-top: 8px; font-size: 12px; color: var(--color-text-tertiary); }
.feedback { margin-top: 16px; padding: 12px; border-radius: var(--radius-md); background: var(--color-danger-light); }
.feedback.correct { background: var(--color-success-light); }
.feedback.exam { background: var(--color-info-light); color: var(--color-info); }
.analysis { margin-top: 8px; color: var(--color-text-secondary); }
textarea { width: 100%; min-height: 80px; padding: 8px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); }
.ai-btn { margin-left: auto; background: var(--color-info); color: #fff; border: none; padding: 8px 14px; border-radius: var(--radius-md); cursor: pointer; }
.ai-btn:disabled { background: var(--color-text-tertiary); cursor: not-allowed; }
.ai-analysis { margin-top: 16px; padding: 12px; border-radius: var(--radius-md); background: var(--color-info-light); border: 1px solid var(--color-info); }
.ai-analysis pre { white-space: pre-wrap; word-break: break-word; margin: 8px 0 0; font-family: inherit; font-size: 14px; line-height: 1.6; }
.ai-error { margin-top: 12px; padding: 8px 12px; border-radius: var(--radius-md); background: var(--color-danger-light); color: var(--color-danger); }
</style>
