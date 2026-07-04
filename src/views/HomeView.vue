<template>
  <div class="home">
    <div class="header">
      <h2>我的题库</h2>
      <button @click="showNew = true">+ 新建题库</button>
    </div>

    <div v-if="lastPractice" class="resume-card" @click="resumePractice">
      <div class="resume-info">
        <div class="resume-label">继续刷题</div>
        <div class="resume-title">{{ lastPractice.bank_name }}</div>
        <div class="resume-meta">第 {{ lastPractice.position }} / {{ lastPractice.total }} 题 · {{ formatTime(lastPractice.saved_at) }}</div>
      </div>
      <div class="resume-arrow">›</div>
    </div>

    <div v-if="bankStore.loading">加载中...</div>
    <div v-else-if="bankStore.banks.length === 0" class="empty">
      <p>还没有题库，点击右上角"新建题库"开始吧</p>
    </div>
    <div v-else class="grid">
      <div v-for="b in bankStore.banks" :key="b.id" class="card">
        <div class="card-header">
          <h3>{{ b.name }}</h3>
          <button class="more-btn" @click="toggleMenu(b.id)">⋯</button>
        </div>
        <p>{{ b.question_count }} 题</p>
        <div class="actions">
          <button class="primary-btn" @click="$router.push(`/practice/${b.id}`)">开始刷题</button>
        </div>
        <div v-if="openMenuId === b.id" class="dropdown-menu" @click.stop>
          <button @click="$router.push(`/wrong/${b.id}`)">错题本</button>
          <button @click="$router.push(`/favorites/${b.id}`)">收藏夹</button>
          <button @click="$router.push(`/import/${b.id}`)">导入题目</button>
          <button @click="exportBank(b)">导出题库</button>
          <button class="danger" @click="del(b.id)">删除题库</button>
        </div>
      </div>
    </div>
    <div v-if="showNew" class="modal">
      <div class="modal-body">
        <h3>新建题库</h3>
        <input v-model="newName" placeholder="题库名称" />
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
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { useBankStore } from '../stores/bank'
import { api } from '../utils/api'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'

interface LastPractice {
  bank_id: number
  bank_name: string
  position: number
  total: number
  saved_at: string
}

const router = useRouter()
const bankStore = useBankStore()
const showNew = ref(false)
const newName = ref('')
const newDesc = ref('')
const lastPractice = ref<LastPractice | null>(null)
const openMenuId = ref<number | null>(null)

function toggleMenu(id: number) {
  openMenuId.value = openMenuId.value === id ? null : id
}

function closeMenu() {
  openMenuId.value = null
}

onMounted(async () => {
  document.addEventListener('click', closeMenu)
  await bankStore.load()
  // 加载最近练习记录，用于"继续刷题"卡片
  try {
    const raw = await api.getSetting('last_practice')
    if (raw) {
      const parsed = JSON.parse(raw) as LastPractice
      // 仅当对应题库仍存在时显示
      if (parsed && typeof parsed.bank_id === 'number' && bankStore.banks.some(b => b.id === parsed.bank_id)) {
        lastPractice.value = parsed
      }
    }
  } catch (e) {
    console.error('加载最近练习记录失败：', e)
  }
})

function formatTime(iso: string): string {
  try {
    const d = new Date(iso)
    return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  } catch {
    return ''
  }
}

function resumePractice() {
  if (lastPractice.value) {
    router.push(`/practice/${lastPractice.value.bank_id}`)
  }
}

async function create() {
  if (!newName.value.trim()) return
  try {
    await bankStore.create(newName.value.trim(), newDesc.value || null)
    showNew.value = false
    newName.value = ''
    newDesc.value = ''
  } catch (e) {
    alert('创建题库失败：' + e)
  }
}
async function del(id: number) {
  if (!confirm('确认删除该题库？')) return
  try {
    await bankStore.remove(id)
    openMenuId.value = null
    // 若删除的正是最近练习题库，清除卡片
    if (lastPractice.value && lastPractice.value.bank_id === id) {
      lastPractice.value = null
      try { await api.setSetting('last_practice', '') } catch {}
    }
  } catch (e) {
    alert('删除题库失败：' + e)
  }
}

// 导出题库为 JSON 文件
async function exportBank(b: { id: number; name: string }) {
  openMenuId.value = null
  try {
    console.log('开始导出题库:', b.id, b.name)
    const jsonStr = await api.exportBank(b.id)
    console.log('获取数据成功, 长度:', jsonStr.length)
    const defaultName = `${b.name}_导出_${new Date().toISOString().slice(0, 10)}.json`
    console.log('打开保存对话框...')
    const filePath = await save({
      defaultPath: defaultName,
      filters: [{ name: 'JSON', extensions: ['json'] }]
    })
    console.log('保存路径:', filePath)
    if (filePath) {
      await writeTextFile(filePath, jsonStr)
      alert('导出成功！已保存到：' + filePath)
    } else {
      console.log('用户取消了保存')
    }
  } catch (e) {
    console.error('导出失败:', e)
    alert('导出失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

onBeforeUnmount(() => {
  document.removeEventListener('click', closeMenu)
})
</script>

<style scoped>
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 16px; }
.card { position: relative; border: 1px solid var(--color-border); border-radius: var(--radius-lg); padding: 16px; background: var(--color-card); }
.card-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 8px; }
.card-header h3 { margin: 0; flex: 1; }
.more-btn { padding: 0 8px; border: none; background: none; cursor: pointer; color: var(--color-text-secondary); font-size: 18px; line-height: 1; border-radius: var(--radius-sm); }
.more-btn:hover { background: var(--color-border-light); }
.card p { margin: 8px 0 12px 0; color: var(--color-text-secondary); font-size: 13px; }
.actions { margin-top: 12px; }
.primary-btn { width: 100%; padding: 8px 12px; border: none; border-radius: var(--radius-md); background: var(--color-primary); color: #fff; cursor: pointer; font-size: 14px; font-weight: 500; }
.primary-btn:hover { background: var(--color-primary-dark); }

.dropdown-menu { position: absolute; top: 40px; right: 12px; min-width: 140px; background: var(--color-card); border: 1px solid var(--color-border); border-radius: var(--radius-md); box-shadow: 0 4px 16px rgba(0,0,0,0.12); padding: 4px; z-index: 10; }
.dropdown-menu button { display: block; width: 100%; text-align: left; padding: 8px 12px; border: none; background: none; cursor: pointer; color: var(--color-text); font-size: 13px; border-radius: var(--radius-sm); }
.dropdown-menu button:hover { background: var(--color-border-light); }
.dropdown-menu button.danger { color: var(--color-danger); }
.dropdown-menu button.danger:hover { background: var(--color-danger-light); }

button { padding: 6px 12px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); cursor: pointer; color: var(--color-text); }
button:hover { background: var(--color-border-light); }
.modal { position: fixed; inset: 0; background: rgba(0,0,0,0.4); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal-body { background: var(--color-card); padding: 24px; border-radius: var(--radius-lg); min-width: 320px; display: flex; flex-direction: column; gap: 8px; color: var(--color-text); }
input, textarea { padding: 8px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-card); color: var(--color-text); }
.empty { text-align: center; padding: 48px; color: var(--color-text-tertiary); }

.resume-card { display: flex; align-items: center; justify-content: space-between; padding: 16px 20px; margin-bottom: 20px; background: var(--color-primary); color: #fff; border-radius: var(--radius-xl); cursor: pointer; box-shadow: 0 2px 8px rgba(66, 184, 131, 0.25); transition: transform 0.15s; }
.resume-card:hover { transform: translateY(-1px); }
.resume-label { font-size: 12px; opacity: 0.9; margin-bottom: 4px; letter-spacing: 1px; }
.resume-title { font-size: 18px; font-weight: bold; }
.resume-meta { font-size: 13px; opacity: 0.9; margin-top: 4px; }
.resume-arrow { font-size: 28px; line-height: 1; }
</style>
