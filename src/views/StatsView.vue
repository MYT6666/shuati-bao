<template>
  <div class="stats">
    <h2>学习统计</h2>

    <div v-if="!loaded" class="loading">加载中...</div>

    <div v-else>
      <!-- 基础统计卡片 -->
      <div class="cards">
        <div class="stat-card">
          <span class="num">{{ stats.total }}</span>
          <label>总题数</label>
        </div>
        <div class="stat-card">
          <span class="num">{{ stats.practiced }}</span>
          <label>已练习</label>
        </div>
        <div class="stat-card highlight">
          <span class="num">{{ accuracy }}%</span>
          <label>正确率</label>
        </div>
        <div class="stat-card">
          <span class="num">{{ correct }}</span>
          <label>答对</label>
        </div>
        <div class="stat-card">
          <span class="num">{{ wrong }}</span>
          <label>答错</label>
        </div>
        <div class="stat-card">
          <span class="num">{{ remaining }}</span>
          <label>未练习</label>
        </div>
      </div>

      <!-- 进度条 -->
      <div class="progress-section">
        <h3>练习进度</h3>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: progressPct + '%' }"></div>
        </div>
        <p class="progress-text">已完成 {{ stats.practiced }} / {{ stats.total }} 题（{{ progressPct }}%）</p>
      </div>

      <!-- 学习热力图（365天） -->
      <div class="heatmap-section">
        <h3>学习热力图（最近 365 天）</h3>
        <div class="heatmap-wrap">
          <div class="heatmap-months">
            <span v-for="m in heatmap.months" :key="m.label" :style="{ gridColumn: m.col + ' / span ' + m.span }">{{ m.label }}</span>
          </div>
          <div class="heatmap-grid">
            <div class="weekday-labels">
              <span></span><span>一</span><span></span><span>三</span><span></span><span>五</span><span></span>
            </div>
            <div v-for="(week, wi) in heatmap.weeks" :key="wi" class="heatmap-week">
              <div
                v-for="(cell, ci) in week"
                :key="ci"
                class="heatmap-cell"
                :class="['level-' + cell.level, { 'today': cell.isToday }]"
                :title="cell.date ? `${cell.date}：${cell.total} 题（对 ${cell.correct}）` : ''"
              ></div>
            </div>
          </div>
          <div class="heatmap-legend">
            <span>少</span>
            <span class="legend-cell level-0"></span>
            <span class="legend-cell level-1"></span>
            <span class="legend-cell level-2"></span>
            <span class="legend-cell level-3"></span>
            <span class="legend-cell level-4"></span>
            <span>多</span>
          </div>
        </div>
        <div class="heatmap-summary">
          共刷题 <b>{{ heatmap.totalCount }}</b> 次 · 连续 <b>{{ heatmap.streakDays }}</b> 天 · 最长 <b>{{ heatmap.maxStreak }}</b> 天
        </div>
      </div>

      <!-- 学习建议 -->
      <div class="tips">
        <h3>学习建议</h3>
        <div v-if="stats.total === 0" class="tip-item">暂无题目，请先导入题库</div>
        <div v-else-if="progressPct < 30" class="tip-item">刚开始练习，加油！建议每天坚持 30 分钟以上</div>
        <div v-else-if="progressPct < 70" class="tip-item">进度过半，继续保持节奏</div>
        <div v-else-if="progressPct < 100" class="tip-item">即将完成全部题目，可重点攻克剩余错题</div>
        <div v-else class="tip-item">已完成全部题目！建议进入错题本复习</div>

        <div v-if="wrong > 0" class="tip-item warn">
          有 {{ wrong }} 题答错过，建议进入
          <RouterLink :to="`/wrong/${bankId}`" class="link">错题本</RouterLink>
          复习
        </div>
        <div v-if="accuracy < 60 && stats.practiced > 10" class="tip-item warn">
          正确率偏低（{{ accuracy }}%），建议放慢节奏，仔细看解析
        </div>
        <div v-if="accuracy >= 90 && stats.practiced > 10" class="tip-item success">
          正确率优秀！可以尝试模拟考试模式挑战自己
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, RouterLink } from 'vue-router'
import { api } from '../utils/api'
import { toastError } from '../utils/toast'

const route = useRoute()
const bankId = Number(route.params.bankId)
const stats = ref({ total: 0, practiced: 0, correct: 0 })
const loaded = ref(false)

const accuracy = computed(() => stats.value.practiced ? Math.round(stats.value.correct / stats.value.practiced * 100) : 0)
const correct = computed(() => stats.value.correct)
const wrong = computed(() => stats.value.practiced - stats.value.correct)
const remaining = computed(() => Math.max(0, stats.value.total - stats.value.practiced))
const progressPct = computed(() => stats.value.total ? Math.round(stats.value.practiced / stats.value.total * 100) : 0)

onMounted(async () => {
  try {
    stats.value = await api.bankStats(bankId)
  } catch (e) {
    toastError('加载统计失败：' + (e instanceof Error ? e.message : String(e)))
  }
  try {
    await loadHeatmap()
  } catch (e) {
    console.error('加载热力图失败：', e)
  } finally {
    loaded.value = true
  }
})

// 热力图数据
interface HeatCell { date: string | null; level: number; total: number; correct: number; isToday: boolean }
interface HeatmapData {
  weeks: HeatCell[][]
  months: { label: string; col: number; span: number }[]
  totalCount: number
  streakDays: number
  maxStreak: number
}
const heatmap = ref<HeatmapData>({ weeks: [], months: [], totalCount: 0, streakDays: 0, maxStreak: 0 })

async function loadHeatmap() {
  const raw = await api.getSetting('daily_records')
  const records: { date: string; total: number; correct: number }[] = raw ? JSON.parse(raw) : []
  const map = new Map(records.map(r => [r.date, r]))

  // 计算 365 天范围：从今天往回数 52 周 + 今天的星期
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const todayDow = (today.getDay() + 6) % 7 // 周一=0
  // 起点：本周日（一周从周日开始，但格子从周一开始排）
  // 我们采用"53 周前开始，每周 7 格（周一到周日）"
  const start = new Date(today)
  start.setDate(today.getDate() - 52 * 7 - todayDow)

  // 找出最大 total 以分 5 档
  let maxTotal = 0
  for (const r of records) if (r.total > maxTotal) maxTotal = r.total

  // 构建 weeks 数组（53 周）
  const weeks: HeatCell[][] = []
  const cursor = new Date(start)
  let col = 0
  let prevMonth = -1
  const months: { label: string; col: number; span: number }[] = []
  while (cursor <= today || col < 53) {
    if (col >= 53) break
    const week: HeatCell[] = []
    for (let dow = 0; dow < 7; dow++) {
      if (cursor > today) {
        week.push({ date: null, level: 0, total: 0, correct: 0, isToday: false })
      } else {
        const iso = `${cursor.getFullYear()}-${String(cursor.getMonth() + 1).padStart(2, '0')}-${String(cursor.getDate()).padStart(2, '0')}`
        const rec = map.get(iso)
        const total = rec?.total ?? 0
        let level = 0
        if (total > 0 && maxTotal > 0) {
          const ratio = total / maxTotal
          if (ratio >= 0.75) level = 4
          else if (ratio >= 0.5) level = 3
          else if (ratio >= 0.25) level = 2
          else level = 1
        }
        const isToday = cursor.getTime() === today.getTime()
        week.push({ date: iso, level, total, correct: rec?.correct ?? 0, isToday })
        if (cursor.getDate() === 1 || (col === 0 && dow === 0)) {
          const label = `${cursor.getMonth() + 1}月`
          if (prevMonth !== cursor.getMonth()) {
            const span = col === 0 ? Math.max(1, col + 1) : 1
            months.push({ label, col: col + 1, span })
            prevMonth = cursor.getMonth()
          }
        }
      }
      cursor.setDate(cursor.getDate() + 1)
    }
    weeks.push(week)
    col++
  }
  // 简化月份标签：每跨月就标一次
  const monthLabels: { label: string; col: number; span: number }[] = []
  let lastSeen = -1
  for (let i = 0; i < weeks.length; i++) {
    const firstDay = weeks[i].find(c => c.date)
    if (firstDay?.date) {
      const m = Number(firstDay.date.split('-')[1])
      if (m !== lastSeen) {
        monthLabels.push({ label: m + '月', col: i + 1, span: 1 })
        lastSeen = m
      } else if (monthLabels.length) {
        monthLabels[monthLabels.length - 1].span++
      }
    }
  }

  // 统计
  let totalCount = 0
  for (const r of records) totalCount += r.total
  // 连续天数（从今天往回数）
  let streak = 0
  const c2 = new Date(today)
  while (true) {
    const iso = `${c2.getFullYear()}-${String(c2.getMonth() + 1).padStart(2, '0')}-${String(c2.getDate()).padStart(2, '0')}`
    const r = map.get(iso)
    if (r && r.total > 0) {
      streak++
      c2.setDate(c2.getDate() - 1)
    } else {
      break
    }
  }
  // 最长连续
  const sortedDates = records.filter(r => r.total > 0).map(r => r.date).sort()
  let maxStreak = 0
  let cur = 0
  let prevDate: Date | null = null
  for (const ds of sortedDates) {
    const d = new Date(ds)
    if (prevDate && (d.getTime() - prevDate.getTime()) === 86400000) {
      cur++
    } else {
      cur = 1
    }
    if (cur > maxStreak) maxStreak = cur
    prevDate = d
  }

  heatmap.value = { weeks, months: monthLabels, totalCount, streakDays: streak, maxStreak }
}
</script>

<style scoped>
.stats { max-width: 800px; }
.loading { text-align: center; padding: 48px; color: var(--color-text-tertiary); }
.cards { display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 24px; }
.stat-card { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 20px 24px; text-align: center; min-width: 110px; flex: 1; }
.stat-card.highlight { background: var(--color-primary); color: #fff; border-color: var(--color-primary); }
.stat-card .num { display: block; font-size: 28px; font-weight: 600; color: var(--color-primary); }
.stat-card.highlight .num { color: #fff; }
.stat-card label { display: block; font-size: 12px; color: var(--color-text-tertiary); margin-top: 6px; }
.stat-card.highlight label { color: rgba(255,255,255,0.8); }

.progress-section { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 16px; margin-bottom: 16px; }
.progress-section h3 { margin: 0 0 12px 0; font-size: 14px; color: var(--color-text-secondary); }
.progress-bar { width: 100%; height: 12px; background: var(--color-border-light); border-radius: 6px; overflow: hidden; }
.progress-fill { height: 100%; background: var(--color-primary); border-radius: 6px; transition: width 0.3s; }
.progress-text { margin: 8px 0 0; font-size: 13px; color: var(--color-text-secondary); }

/* 热力图 */
.heatmap-section { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 16px 20px; margin-bottom: 16px; }
.heatmap-section h3 { margin: 0 0 12px 0; font-size: 15px; }
.heatmap-wrap { overflow-x: auto; padding-bottom: 4px; }
.heatmap-months { display: grid; grid-template-columns: 24px repeat(53, 12px); gap: 3px; font-size: 11px; color: var(--color-text-tertiary); margin-bottom: 4px; }
.heatmap-months span { white-space: nowrap; }
.heatmap-grid { display: flex; gap: 3px; }
.weekday-labels { display: flex; flex-direction: column; gap: 3px; width: 24px; flex-shrink: 0; font-size: 10px; color: var(--color-text-tertiary); padding-top: 1px; }
.weekday-labels span { height: 12px; line-height: 12px; }
.heatmap-week { display: flex; flex-direction: column; gap: 3px; }
.heatmap-cell { width: 12px; height: 12px; border-radius: 2px; background: var(--color-border-light); transition: transform 0.1s; }
.heatmap-cell:hover { transform: scale(1.4); outline: 1px solid var(--color-primary); }
.heatmap-cell.today { outline: 1.5px solid #2563eb; outline-offset: 1px; }
.heatmap-cell.level-0 { background: #ebedf0; }
.heatmap-cell.level-1 { background: #c6e48b; }
.heatmap-cell.level-2 { background: #7bc96f; }
.heatmap-cell.level-3 { background: #239a3b; }
.heatmap-cell.level-4 { background: #196127; }
[data-theme="dark"] .heatmap-cell.level-0 { background: #2d333b; }
[data-theme="dark"] .heatmap-cell.level-1 { background: #0e4429; }
[data-theme="dark"] .heatmap-cell.level-2 { background: #006d32; }
[data-theme="dark"] .heatmap-cell.level-3 { background: #26a641; }
[data-theme="dark"] .heatmap-cell.level-4 { background: #39d353; }
.heatmap-legend { display: flex; align-items: center; gap: 4px; margin-top: 12px; font-size: 11px; color: var(--color-text-tertiary); }
.legend-cell { width: 12px; height: 12px; border-radius: 2px; display: inline-block; }
.legend-cell.level-0 { background: #ebedf0; }
.legend-cell.level-1 { background: #c6e48b; }
.legend-cell.level-2 { background: #7bc96f; }
.legend-cell.level-3 { background: #239a3b; }
.legend-cell.level-4 { background: #196127; }
[data-theme="dark"] .legend-cell.level-0 { background: #2d333b; }
[data-theme="dark"] .legend-cell.level-1 { background: #0e4429; }
[data-theme="dark"] .legend-cell.level-2 { background: #006d32; }
[data-theme="dark"] .legend-cell.level-3 { background: #26a641; }
[data-theme="dark"] .legend-cell.level-4 { background: #39d353; }
.heatmap-summary { font-size: 12px; color: var(--color-text-secondary); margin-top: 8px; }
.heatmap-summary b { color: var(--color-primary); font-weight: 600; }

.tips { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 16px 20px; }
.tips h3 { margin: 0 0 12px 0; font-size: 14px; color: var(--color-text-secondary); }
.tip-item { padding: 10px 0; border-bottom: 1px solid var(--color-border-light); font-size: 14px; color: var(--color-text); }
.tip-item:last-child { border-bottom: none; }
.tip-item.warn { color: var(--color-warning); }
.tip-item.success { color: var(--color-success); }
.link { color: var(--color-primary); text-decoration: none; font-weight: 500; }
.link:hover { text-decoration: underline; }
</style>
