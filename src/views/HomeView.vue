<template>
  <div class="home">
    <div class="header">
      <div>
        <h2>我的题库</h2>
        <div class="header-sub">
          共 <b>{{ bankStore.banks.length }}</b> 个题库 ·
          <b>{{ totalQuestions }}</b> 道题 ·
          已掌握 <b>{{ totalMastered }}</b> 道
        </div>
      </div>
      <button class="new-bank-btn" @click="showNew = true">+ 新建题库</button>
    </div>

    <!-- 每日学习卡片（学习类 App 常见激励） -->
    <div v-if="todayStats.total > 0" class="daily-card">
      <div class="daily-left">
        <div class="daily-icon">🔥</div>
        <div>
          <div class="daily-title">今天已刷 <b>{{ todayStats.total }}</b> 题</div>
          <div class="daily-sub">正确率 {{ todayStats.accuracy }}% · 连续刷题 <b>{{ streakDays }}</b> 天</div>
        </div>
      </div>
      <div class="daily-progress">
        <div class="daily-progress-bar">
          <div class="daily-progress-fill" :style="{ width: todayStats.accuracy + '%' }"></div>
        </div>
        <div class="daily-progress-label">今日正确率</div>
      </div>
    </div>

    <div v-if="lastPractice" class="resume-card" @click="resumePractice">
      <div class="resume-info">
        <div class="resume-label">继续刷题</div>
        <div class="resume-title">{{ lastPractice.bank_name }}</div>
        <div class="resume-meta">第 {{ lastPractice.position }} / {{ lastPractice.total }} 题 · {{ formatTime(lastPractice.saved_at) }}</div>
      </div>
      <div class="resume-arrow">›</div>
    </div>

    <div v-if="bankStore.loading" class="skeleton-grid">
      <div v-for="i in 3" :key="i" class="skeleton-card"></div>
    </div>
    <div v-else-if="bankStore.banks.length === 0" class="empty">
      <div class="empty-icon">📚</div>
      <p class="empty-title">还没有题库</p>
      <p class="empty-tip">点击右上角"新建题库"开始你的刷题之旅</p>
      <button class="empty-action" @click="showNew = true">+ 新建第一个题库</button>
    </div>
    <div v-else class="grid">
      <div v-for="b in bankStore.banks" :key="b.id" class="card" :class="{ 'has-progress': statsFor(b.id)?.practiced }">
        <div class="card-header">
          <h3>{{ b.name }}</h3>
          <button class="more-btn" @click.stop="toggleMenu(b.id)">⋯</button>
        </div>
        <div class="card-meta">
          <span class="count-pill">📝 {{ b.question_count }} 题</span>
          <span v-if="statsFor(b.id)" class="accuracy-pill" :class="accuracyClass(statsFor(b.id)!.accuracy)">
            ✓ {{ statsFor(b.id)!.accuracy }}%
          </span>
        </div>

        <!-- 进度条 -->
        <div v-if="statsFor(b.id)?.practiced" class="progress-row">
          <div class="progress-bar">
            <div class="progress-fill" :style="{ width: progressPct(b.id) + '%' }"></div>
          </div>
          <div class="progress-text">{{ statsFor(b.id)!.practiced }} / {{ b.question_count }}</div>
        </div>

        <div class="actions">
          <button class="primary-btn" @click="$router.push(`/practice/${b.id}`)">开始刷题</button>
        </div>

        <div v-if="openMenuId === b.id" class="dropdown-menu" @click.stop>
          <button @click="$router.push(`/wrong/${b.id}`)">📕 错题本</button>
          <button @click="$router.push(`/favorites/${b.id}`)">⭐ 收藏夹</button>
          <button @click="$router.push(`/import/${b.id}`)">📥 导入题目</button>
          <button @click="exportBank(b)">📤 导出题库</button>
          <button class="danger" @click="del(b.id)">🗑 删除题库</button>
        </div>
      </div>
    </div>

    <div v-if="showNew" class="modal" @click.self="showNew = false">
      <div class="modal-body">
        <h3>新建题库</h3>
        <input v-model="newName" placeholder="题库名称" @keyup.enter="create" />
        <textarea v-model="newDesc" placeholder="描述（可选）"></textarea>
        <div class="modal-actions">
          <button @click="create">确定</button>
          <button @click="showNew = false">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useBankStore } from '../stores/bank'
import { api } from '../utils/api'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import { toastSuccess, toastError } from '../utils/toast'

interface LastPractice {
  bank_id: number
  bank_name: string
  position: number
  total: number
  saved_at: string
}
interface Stats { total: number; practiced: number; correct: number; accuracy: number }
interface TodayStats { total: number; correct: number; accuracy: number }
interface DailyRecord { date: string; total: number; correct: number }

const router = useRouter()
const bankStore = useBankStore()
const showNew = ref(false)
const newName = ref('')
const newDesc = ref('')
const lastPractice = ref<LastPractice | null>(null)
const openMenuId = ref<number | null>(null)
const bankStatsMap = ref<Map<number, Stats>>(new Map())
const todayStats = ref<TodayStats>({ total: 0, correct: 0, accuracy: 0 })
const streakDays = ref(0)

const totalQuestions = computed(() => bankStore.banks.reduce((s, b) => s + b.question_count, 0))
const totalMastered = computed(() => {
  let m = 0
  for (const s of bankStatsMap.value.values()) m += s.practiced // 近似用已练习数代替掌握数
  return m
})

function toggleMenu(id: number) {
  openMenuId.value = openMenuId.value === id ? null : id
}
function closeMenu() { openMenuId.value = null }

function statsFor(id: number): Stats | undefined {
  return bankStatsMap.value.get(id)
}

function progressPct(id: number): number {
  const b = bankStore.banks.find(x => x.id === id)
  const s = bankStatsMap.value.get(id)
  if (!b || !s || b.question_count === 0) return 0
  return Math.min(100, Math.round((s.practiced / b.question_count) * 100))
}

function accuracyClass(accuracy: number): string {
  if (accuracy >= 80) return 'high'
  if (accuracy >= 60) return 'mid'
  return 'low'
}

function todayISO(): string {
  const d = new Date()
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
}

onMounted(async () => {
  document.addEventListener('click', closeMenu)
  await bankStore.load()
  // 加载每个题库统计
  await Promise.all(bankStore.banks.map(async b => {
    try {
      const s = await api.bankStats(b.id)
      bankStatsMap.value.set(b.id, { ...s, accuracy: s.practiced > 0 ? Math.round((s.correct / s.practiced) * 100) : 0 })
    } catch { /* ignore */ }
  }))
  // 计算今日统计 & 连续天数
  try {
    const raw = await api.getSetting('daily_records')
    const records: DailyRecord[] = raw ? JSON.parse(raw) : []
    const today = todayISO()
    const todayRec = records.find(r => r.date === today)
    if (todayRec) {
      todayStats.value = {
        total: todayRec.total,
        correct: todayRec.correct,
        accuracy: todayRec.total > 0 ? Math.round((todayRec.correct / todayRec.total) * 100) : 0,
      }
    }
    // 连续天数：从今天往回数，每天都有记录
    let streak = 0
    const sorted = [...records].sort((a, b) => b.date.localeCompare(a.date))
    let cursor = new Date()
    for (const r of sorted) {
      const iso = `${cursor.getFullYear()}-${String(cursor.getMonth() + 1).padStart(2, '0')}-${String(cursor.getDate()).padStart(2, '0')}`
      if (r.date === iso && r.total > 0) {
        streak++
        cursor.setDate(cursor.getDate() - 1)
      } else if (r.date < iso) {
        break
      }
    }
    streakDays.value = streak
  } catch (e) { console.error('加载每日统计失败：', e) }
  // 加载最近练习记录
  try {
    const raw = await api.getSetting('last_practice')
    if (raw) {
      const parsed = JSON.parse(raw) as LastPractice
      if (parsed && typeof parsed.bank_id === 'number' && bankStore.banks.some(b => b.id === parsed.bank_id)) {
        lastPractice.value = parsed
      }
    }
  } catch (e) { console.error('加载最近练习记录失败：', e) }
})

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  } catch { return '' }
}

function resumePractice() {
  if (lastPractice.value) router.push(`/practice/${lastPractice.value.bank_id}`)
}

async function create() {
  if (!newName.value.trim()) return
  try {
    const created = await bankStore.create(newName.value.trim(), newDesc.value || null)
    showNew.value = false
    newName.value = ''
    newDesc.value = ''
    if (created && created.id) {
      if (confirm('题库创建成功！是否立即导入题目？\n（点"取消"稍后从题库卡片 ⋯ 菜单里导入）')) {
        router.push(`/import/${created.id}`)
      }
    }
  } catch (e) {
    toastError('创建题库失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function del(id: number) {
  if (!confirm('确认删除该题库？此操作不可恢复。')) return
  try {
    await bankStore.remove(id)
    openMenuId.value = null
    bankStatsMap.value.delete(id)
    if (lastPractice.value && lastPractice.value.bank_id === id) {
      lastPractice.value = null
      try { await api.setSetting('last_practice', '') } catch {}
    }
    toastSuccess('题库已删除')
  } catch (e) {
    toastError('删除题库失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function exportBank(b: { id: number; name: string }) {
  openMenuId.value = null
  try {
    const jsonStr = await api.exportBank(b.id)
    const defaultName = `${b.name}_导出_${new Date().toISOString().slice(0, 10)}.json`
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: 'JSON', extensions: ['json'] }]
    })
    if (filePath) {
      await writeTextFile(filePath, jsonStr)
      toastSuccess('导出成功：' + filePath)
    }
  } catch (e) {
    toastError('导出失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

onBeforeUnmount(() => document.removeEventListener('click', closeMenu))
</script>

<style scoped>
.header { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 20px; gap: 16px; }
.header h2 { margin: 0 0 4px 0; font-size: 24px; }
.header-sub { font-size: 13px; color: var(--color-text-secondary); }
.header-sub b { color: var(--color-primary); font-weight: 600; }

.new-bank-btn { padding: 9px 18px; background: var(--color-primary); color: #fff; border: none; border-radius: var(--radius-md); font-size: 14px; cursor: pointer; font-weight: 500; transition: background 0.15s, transform 0.1s; white-space: nowrap; }
.new-bank-btn:hover { background: var(--color-primary-dark); transform: translateY(-1px); }

/* 每日激励卡 */
.daily-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 20px;
  margin-bottom: 16px;
  background: linear-gradient(135deg, #fff7ed 0%, #fef3c7 100%);
  border: 1px solid #fde68a;
  border-radius: var(--radius-lg);
  box-shadow: 0 2px 6px rgba(245, 158, 11, 0.08);
}
.daily-left { display: flex; align-items: center; gap: 14px; }
.daily-icon { font-size: 36px; }
.daily-title { font-size: 16px; color: #92400e; font-weight: 600; }
.daily-title b { color: #c2410c; font-size: 18px; }
.daily-sub { font-size: 13px; color: #b45309; margin-top: 4px; }
.daily-sub b { color: #c2410c; }
.daily-progress { min-width: 160px; }
.daily-progress-bar { height: 6px; background: rgba(146, 64, 14, 0.15); border-radius: 3px; overflow: hidden; }
.daily-progress-fill { height: 100%; background: linear-gradient(90deg, #f59e0b, #f97316); border-radius: 3px; transition: width 0.4s; }
.daily-progress-label { font-size: 11px; color: #b45309; margin-top: 4px; text-align: right; }

.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 18px; }
.card { position: relative; border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: 18px; background: var(--color-card); transition: transform 0.18s, box-shadow 0.18s, border-color 0.18s; cursor: default; display: flex; flex-direction: column; }
.card:hover { transform: translateY(-3px); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08); border-color: var(--color-primary-light); }
.card.has-progress { border-left: 3px solid var(--color-primary); }
.card-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; margin-bottom: 8px; }
.card-header h3 { margin: 0; flex: 1; font-size: 16px; line-height: 1.4; word-break: break-all; }
.more-btn { padding: 4px 8px; border: none; background: none; cursor: pointer; color: var(--color-text-secondary); font-size: 20px; line-height: 1; border-radius: var(--radius-sm); transition: background 0.12s; }
.more-btn:hover { background: var(--color-border-light); }
.card-meta { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; margin-bottom: 12px; }
.count-pill, .accuracy-pill {
  display: inline-flex; align-items: center; gap: 3px;
  padding: 3px 10px; border-radius: 12px;
  font-size: 12px; font-weight: 500;
}
.count-pill { background: var(--color-primary-light); color: var(--color-primary); }
.accuracy-pill { background: #dcfce7; color: #15803d; }
.accuracy-pill.mid { background: #fef3c7; color: #b45309; }
.accuracy-pill.low { background: #fee2e2; color: #b91c1c; }

.progress-row { margin-bottom: 14px; }
.progress-bar { height: 6px; background: var(--color-border-light); border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: linear-gradient(90deg, var(--color-primary), var(--color-primary-dark)); border-radius: 3px; transition: width 0.4s; }
.progress-text { font-size: 11px; color: var(--color-text-tertiary); margin-top: 4px; text-align: right; }

.actions { margin-top: auto; }
.primary-btn { width: 100%; padding: 10px 12px; border: none; border-radius: var(--radius-md); background: var(--color-primary); color: #fff; cursor: pointer; font-size: 14px; font-weight: 500; transition: background 0.15s, transform 0.1s; }
.primary-btn:hover { background: var(--color-primary-dark); transform: translateY(-1px); }

.dropdown-menu { position: absolute; top: 50px; right: 12px; min-width: 160px; background: var(--color-card); border: 1px solid var(--color-border); border-radius: var(--radius-md); box-shadow: 0 4px 16px rgba(0,0,0,0.12); padding: 4px; z-index: 10; }
.dropdown-menu button { display: flex; align-items: center; width: 100%; text-align: left; padding: 8px 12px; border: none; background: none; cursor: pointer; color: var(--color-text); font-size: 13px; border-radius: var(--radius-sm); gap: 8px; }
.dropdown-menu button:hover { background: var(--color-border-light); }
.dropdown-menu button.danger { color: var(--color-danger); }
.dropdown-menu button.danger:hover { background: var(--color-danger-light); }

.modal { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 100; animation: fadeIn 0.15s; }
.modal-body { background: var(--color-card); padding: 24px; border-radius: var(--radius-lg); min-width: 340px; display: flex; flex-direction: column; gap: 10px; color: var(--color-text); box-shadow: 0 8px 32px rgba(0,0,0,0.2); }
.modal-body h3 { margin: 0 0 4px 0; }
.modal-body input, .modal-body textarea { padding: 8px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-bg); color: var(--color-text); font-family: inherit; font-size: 14px; }
.modal-body textarea { min-height: 60px; resize: vertical; }
.modal-body input:focus, .modal-body textarea:focus { outline: none; border-color: var(--color-primary); }
.modal-actions { display: flex; gap: 8px; justify-content: flex-end; margin-top: 8px; }
.modal-actions button { padding: 7px 16px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); cursor: pointer; font-size: 14px; }
.modal-actions button:first-child { background: var(--color-primary); color: #fff; border-color: var(--color-primary); }
.modal-actions button:first-child:hover { background: var(--color-primary-dark); }

.empty { text-align: center; padding: 80px 24px; color: var(--color-text-tertiary); }
.empty-icon { font-size: 72px; margin-bottom: 16px; opacity: 0.5; }
.empty-title { font-size: 18px; color: var(--color-text-secondary); margin: 0 0 8px 0; }
.empty-tip { font-size: 14px; margin: 0 0 20px 0; }
.empty-action { padding: 10px 24px; background: var(--color-primary); color: #fff; border: none; border-radius: var(--radius-md); font-size: 14px; cursor: pointer; font-weight: 500; }
.empty-action:hover { background: var(--color-primary-dark); }

.resume-card { display: flex; align-items: center; justify-content: space-between; padding: 18px 22px; margin-bottom: 16px; background: linear-gradient(135deg, var(--color-primary) 0%, var(--color-primary-dark) 100%); color: #fff; border-radius: var(--radius-xl); cursor: pointer; box-shadow: 0 4px 12px rgba(66, 184, 131, 0.3); transition: transform 0.15s, box-shadow 0.15s; }
.resume-card:hover { transform: translateY(-1px); box-shadow: 0 6px 18px rgba(66, 184, 131, 0.4); }
.resume-label { font-size: 12px; opacity: 0.9; margin-bottom: 4px; letter-spacing: 1px; }
.resume-title { font-size: 18px; font-weight: bold; }
.resume-meta { font-size: 13px; opacity: 0.9; margin-top: 4px; }
.resume-arrow { font-size: 32px; line-height: 1; opacity: 0.8; }

/* 加载骨架 */
.skeleton-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 18px; }
.skeleton-card { height: 160px; background: linear-gradient(90deg, var(--color-border-light) 0%, var(--color-card) 50%, var(--color-border-light) 100%); background-size: 200% 100%; border-radius: var(--radius-lg); animation: shimmer 1.4s infinite; }

@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
</style>
