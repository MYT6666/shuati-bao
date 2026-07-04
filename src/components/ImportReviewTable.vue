<template>
  <div class="table-wrap">
    <table>
      <thead>
        <tr><th>题号</th><th>题型</th><th>题干</th><th>选项</th><th>答案</th><th>置信度</th></tr>
      </thead>
      <tbody>
        <tr v-for="(q, i) in visibleList" :key="q.id" :class="{ low: q.confidence < 0.6 }">
          <td>{{ q.source_index ?? i + 1 }}</td>
          <td>{{ typeLabel(q.type) }}</td>
          <td><textarea v-model="q.stem" @change="$emit('update', q)"></textarea></td>
          <td>{{ q.options }}</td>
          <td><input v-model="q.answer" @change="$emit('update', q)" /></td>
          <td>{{ (q.confidence * 100).toFixed(0) }}%</td>
        </tr>
      </tbody>
    </table>
    <div v-if="list.length > pageSize" class="pager">
      <span class="pager-info">已显示 {{ visibleList.length }} / {{ list.length }} 题（仅渲染前 {{ visibleList.length }} 行避免卡顿）</span>
      <button v-if="visibleCount < list.length" @click="loadMore">显示更多（+{{ nextBatch }}）</button>
      <button v-if="visibleCount > pageSize" @click="showLess">收起</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { Question } from '../utils/api'

const props = defineProps<{ list: Question[] }>()
defineEmits<{ (e: 'update', q: Question): void }>()

// 分页：默认只渲染前 100 题，避免大量 textarea 撑爆 webview 内存
const PAGE_STEP = 100
const pageSize = PAGE_STEP
const visibleCount = ref(pageSize)

const visibleList = computed(() => props.list.slice(0, visibleCount.value))
const nextBatch = computed(() => Math.min(PAGE_STEP, props.list.length - visibleCount.value))

function loadMore() {
  visibleCount.value = Math.min(visibleCount.value + PAGE_STEP, props.list.length)
}
function showLess() {
  visibleCount.value = pageSize
}

function typeLabel(t: string) {
  return { single: '单选', multi: '多选', judge: '判断', blank: '填空', qa: '问答' }[t] || t
}
</script>

<style scoped>
.table-wrap { overflow: auto; max-height: 60vh; }
table { border-collapse: collapse; width: 100%; }
th, td { border: 1px solid var(--color-border); padding: 6px; text-align: left; font-size: 13px; color: var(--color-text); }
tr.low { background: var(--color-danger-light); }
tr.low td { color: var(--color-danger); font-weight: 500; }
tr.low::before { content: '⚠'; }
textarea, input { width: 100%; border: 1px solid var(--color-border-light); padding: 4px; background: var(--color-card); color: var(--color-text); }
.pager { display: flex; align-items: center; gap: 12px; padding: 10px 8px; background: var(--color-bg); border-top: 1px solid var(--color-border-light); position: sticky; bottom: 0; }
.pager-info { color: var(--color-text-secondary); font-size: 13px; flex: 1; }
.pager button { padding: 6px 14px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-card); cursor: pointer; font-size: 13px; color: var(--color-text); }
.pager button:hover { background: var(--color-primary-light); border-color: var(--color-primary); }
</style>
