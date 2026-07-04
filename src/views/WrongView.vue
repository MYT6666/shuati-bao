<template>
  <div class="wrong">
    <h2>错题本（{{ wrongIds.length }} 题）</h2>

    <div class="tabs">
      <button :class="{ active: tab === 'pending' }" @click="tab = 'pending'">待重练（{{ wrongIds.length }}）</button>
      <button :class="{ active: tab === 'mastered' }" @click="tab = 'mastered'">已掌握（{{ masteredIds.length }}）</button>
    </div>

    <div v-if="tab === 'pending'">
      <div class="actions-bar">
        <button v-if="wrongIds.length && !practicing" @click="start">错题重练</button>
        <button v-if="practicing" @click="exitPractice">退出重练</button>
      </div>

      <QuestionCard
        v-if="practicing && current"
        :key="current.id"
        :question="current"
        :index="idx"
        :favorited="favoriteIds.has(current.id)"
        @answered="onAnswered"
        @next="next"
        @toggle-favorite="onToggleFavorite"
      />

      <div v-if="practicing && current" class="quick-actions">
        <button class="master-btn" @click="markMastered(current.id)">标记已掌握</button>
      </div>

      <div v-else-if="!wrongIds.length" class="empty">暂无错题，继续加油！</div>

      <!-- 错题列表 -->
      <ul v-else class="wrong-list">
        <li
          v-for="(id, index) in wrongIds"
          :key="id"
          @click="jumpToQuestion(id)"
          class="wrong-item"
          :class="{ active: practicing && current && current.id === id }"
        >
          <span class="item-index">{{ index + 1 }}</span>
          <span class="item-preview">{{ getQuestionPreview(id) }}</span>
          <button class="quick-master-btn" title="标记已掌握" @click.stop="markMastered(id)">✓ 掌握</button>
        </li>
      </ul>
    </div>

    <div v-else>
      <div v-if="!masteredIds.length" class="empty">还没有已掌握的错题</div>
      <div v-else>
        <p class="hint">已掌握 {{ masteredIds.length }} 题</p>
        <ul class="mastered-list">
          <li v-for="id in masteredIds" :key="id">
            <span>{{ getQuestionPreview(id) }}</span>
            <button class="restore-btn" @click="restoreToPending(id)">放回错题</button>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { api, Question } from '../utils/api'
import QuestionCard from '../components/QuestionCard.vue'

const route = useRoute()
const bankId = Number(route.params.bankId)
const allQuestions = ref<Question[]>([])
const wrongIds = ref<number[]>([])
const masteredIds = ref<number[]>([])
const favoriteIds = ref<Set<number>>(new Set())
const practicing = ref(false)
const queue = ref<number[]>([])
const idx = ref(0)
const current = ref<Question | null>(null)
const tab = ref<'pending' | 'mastered'>('pending')

onMounted(async () => {
  await loadData()
})

async function loadData() {
  try {
    allQuestions.value = await api.listQuestions(bankId)
    const [wrong, mastered, favs] = await Promise.all([
      api.listWrong(bankId),
      api.listMastered(bankId),
      api.listFavorites(bankId),
    ])
    wrongIds.value = wrong
    masteredIds.value = mastered
    favoriteIds.value = new Set(favs)
  } catch (e) {
    alert('加载错题失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

function start() {
  queue.value = [...wrongIds.value]
  idx.value = 0
  practicing.value = true
  loadCurrent()
}

// 点击列表中的某道题，直接跳到该题练习
function jumpToQuestion(questionId: number) {
  // 找到该题在错题列表中的位置
  queue.value = [...wrongIds.value]
  const pos = queue.value.indexOf(questionId)
  if (pos >= 0) {
    idx.value = pos
    practicing.value = true
    loadCurrent()
  }
}

function exitPractice() {
  practicing.value = false
  current.value = null
}

function loadCurrent() {
  while (idx.value < queue.value.length) {
    const id = queue.value[idx.value]
    const q = allQuestions.value.find(q => q.id === id)
    if (q) {
      current.value = q
      return
    }
    idx.value++
  }
  current.value = null
  practicing.value = false
}

async function next() {
  if (idx.value < queue.value.length - 1) { idx.value++; loadCurrent() }
  else {
    practicing.value = false
    await loadData()
  }
}

async function onAnswered(payload: { correct: boolean; answer: string; duration_ms: number | null }) {
  if (current.value) {
    try {
      await api.recordPractice({ bank_id: bankId, question_id: current.value.id, user_answer: payload.answer, is_correct: payload.correct, duration_ms: payload.duration_ms })
    } catch (e) {
      console.error('记录练习失败：', e)
    }
  }
}

async function onToggleFavorite() {
  if (!current.value) return
  try {
    const nowFav = await api.toggleFavorite(bankId, current.value.id)
    const next = new Set(favoriteIds.value)
    if (nowFav) next.add(current.value.id)
    else next.delete(current.value.id)
    favoriteIds.value = next
  } catch (e) {
    console.error('切换收藏失败：', e)
  }
}

async function markMastered(questionId: number) {
  try {
    await api.markWrongMastered(bankId, questionId)
    await loadData()
    // 自动跳下一题
    if (practicing.value) {
      await next()
    }
  } catch (e) {
    alert('标记失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function restoreToPending(questionId: number) {
  // 把已掌握的题重新答错就能回 pending；这里通过 recordPractice(false) 实现
  try {
    await api.recordPractice({ bank_id: bankId, question_id: questionId, user_answer: '', is_correct: false, duration_ms: null })
    await loadData()
  } catch (e) {
    alert('放回失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

function getQuestionPreview(id: number): string {
  const q = allQuestions.value.find(q => q.id === id)
  if (!q) return `#${id}`
  return q.stem.length > 50 ? q.stem.slice(0, 50) + '...' : q.stem
}
</script>

<style scoped>
.wrong { max-width: 800px; }
.tabs { display: flex; gap: 4px; margin-bottom: 16px; border-bottom: 1px solid var(--color-border); }
.tabs button { background: none; border: none; padding: 8px 16px; cursor: pointer; color: var(--color-text-secondary); border-bottom: 2px solid transparent; }
.tabs button.active { color: var(--color-primary); border-bottom-color: var(--color-primary); }
.actions-bar { margin-bottom: 16px; }
.empty { text-align: center; padding: 48px; color: var(--color-text-tertiary); }
.hint { color: var(--color-text-tertiary); font-size: 13px; padding: 16px 0; }
button { padding: 8px 16px; border: 1px solid var(--color-border); border-radius: var(--radius-md); cursor: pointer; background: var(--color-card); color: var(--color-text); }
button:hover { background: var(--color-border-light); }
.quick-actions { margin-top: 12px; }
.master-btn { background: var(--color-success-light); border-color: var(--color-success); color: var(--color-success); padding: 6px 14px; border-radius: var(--radius-md); cursor: pointer; }
.master-btn:hover { background: var(--color-success); color: #fff; }
.mastered-list { list-style: none; padding: 0; }
.mastered-list li { display: flex; justify-content: space-between; align-items: center; padding: 12px; border: 1px solid var(--color-border-light); border-radius: var(--radius-md); margin-bottom: 8px; background: var(--color-card); }
.restore-btn { background: var(--color-warning-light); border-color: var(--color-warning); color: var(--color-warning); padding: 4px 10px; font-size: 12px; }
.restore-btn:hover { background: var(--color-warning); color: #fff; }
/* 错题列表 */
.wrong-list { list-style: none; padding: 0; margin: 0; }
.wrong-item { display: flex; align-items: center; gap: 12px; padding: 12px 16px; border: 1px solid var(--color-border-light); border-radius: var(--radius-md); margin-bottom: 8px; background: var(--color-card); cursor: pointer; transition: border-color 0.15s, background 0.15s; }
.wrong-item:hover { border-color: var(--color-primary); background: var(--color-primary-light); }
.wrong-item.active { border-color: var(--color-primary); background: var(--color-primary-light); }
.item-index { display: inline-flex; align-items: center; justify-content: center; width: 26px; height: 26px; border-radius: 50%; background: var(--color-danger-light); color: var(--color-danger); font-size: 13px; font-weight: 600; flex-shrink: 0; }
.item-preview { flex: 1; font-size: 14px; color: var(--color-text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.quick-master-btn { padding: 4px 10px; border: 1px solid var(--color-success); border-radius: var(--radius-sm); background: var(--color-success-light); color: var(--color-success); cursor: pointer; font-size: 12px; white-space: nowrap; flex-shrink: 0; opacity: 0; transition: opacity 0.15s; }
.wrong-item:hover .quick-master-btn, .wrong-item.active .quick-master-btn { opacity: 1; }
.quick-master-btn:hover { background: var(--color-success); color: #fff; }
</style>
