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
    alert('加载统计失败：' + (e instanceof Error ? e.message : String(e)))
  } finally {
    loaded.value = true
  }
})
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

.tips { background: var(--color-card); border: 1px solid var(--color-border-light); border-radius: var(--radius-lg); padding: 16px; }
.tips h3 { margin: 0 0 12px 0; font-size: 14px; color: var(--color-text-secondary); }
.tip-item { padding: 10px 0; border-bottom: 1px solid var(--color-border-light); font-size: 14px; color: var(--color-text); }
.tip-item:last-child { border-bottom: none; }
.tip-item.warn { color: var(--color-warning); }
.tip-item.success { color: var(--color-success); }
.link { color: var(--color-primary); text-decoration: none; font-weight: 500; }
.link:hover { text-decoration: underline; }
</style>
