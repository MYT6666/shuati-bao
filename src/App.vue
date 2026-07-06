<template>
  <div class="app">
    <aside class="sidebar">
      <div class="logo">
        <img src="/logo-256.png?v=0.1.3" class="logo-img" alt="刷题宝" />
        <span>刷题宝</span>
      </div>

      <div class="nav-group">
        <div class="nav-group-title">全局</div>
        <nav>
          <RouterLink to="/"><span class="nav-icon">📖</span><span>题库</span></RouterLink>
          <RouterLink to="/settings"><span class="nav-icon">⚙️</span><span>设置</span></RouterLink>
        </nav>
      </div>

      <div v-if="currentBank" class="nav-group">
        <div class="nav-group-title">当前题库</div>
        <div class="bank-name">{{ currentBank.name }}</div>
        <nav>
          <RouterLink :to="`/practice/${currentBank.id}`"><span class="nav-icon">✏️</span><span>练习</span></RouterLink>
          <RouterLink :to="`/wrong/${currentBank.id}`"><span class="nav-icon">❌</span><span>错题本</span></RouterLink>
          <RouterLink :to="`/favorites/${currentBank.id}`"><span class="nav-icon">⭐</span><span>收藏</span></RouterLink>
          <RouterLink :to="`/stats/${currentBank.id}`"><span class="nav-icon">📊</span><span>统计</span></RouterLink>
        </nav>
      </div>

      <div class="sidebar-footer">
        <button class="donate-btn" @click="showDonate = true" title="支持作者">
          <span class="coffee">💝</span>
          <span class="donate-text">支持作者</span>
        </button>
        <div class="app-actions">
          <button class="app-action-btn" @click="showFeedback = true" title="意见反馈">💌</button>
          <button class="app-action-btn" @click="handleRestart" title="重启应用">🔄</button>
          <button class="app-action-btn danger" @click="handleQuit" title="关闭应用">✕</button>
        </div>
      </div>
    </aside>
    <main class="content"><RouterView :key="route.fullPath" /></main>

    <DonateDialog :visible="showDonate" @close="showDonate = false" />
    <FeedbackDialog :visible="showFeedback" @close="showFeedback = false" />
    <Toast />
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { RouterView, RouterLink, useRoute } from 'vue-router'
import { useBankStore } from './stores/bank'
import { api } from './utils/api'
import { autoCheckOnStartup } from './utils/updater'
import DonateDialog from './components/DonateDialog.vue'
import FeedbackDialog from './components/FeedbackDialog.vue'
import Toast from './components/Toast.vue'

const route = useRoute()
const bankStore = useBankStore()
const currentBank = ref<{ id: number; name: string } | null>(null)
const showDonate = ref(false)
const showFeedback = ref(false)

async function refreshBankNav() {
  const bankId = Number(route.params.bankId)
  if (!bankId) {
    currentBank.value = null
    return
  }
  // 确保题库列表已加载
  if (!bankStore.banks.length) {
    try { await bankStore.load() } catch (e) { /* ignore */ }
  }
  const bank = bankStore.banks.find(b => b.id === bankId)
  currentBank.value = bank ? { id: bank.id, name: bank.name } : null
}

watch(() => route.params.bankId, refreshBankNav, { immediate: true })

// 启动 3 秒后后台检查更新
onMounted(() => {
  autoCheckOnStartup().catch(e => console.error('启动检查更新失败：', e))
})

// @ts-ignore
async function handleRestart() {
  if (!confirm('确认重启应用吗？')) return
  try {
    await api.restartApp()
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    if (msg) {
      const div = document.createElement('div')
      div.style.cssText = 'position:fixed;top:24px;right:24px;padding:10px 16px;background:#fee2e2;color:#b91c1c;border-radius:6px;z-index:9999;box-shadow:0 4px 12px rgba(0,0,0,0.15);max-width:340px;'
      div.textContent = '重启失败：' + msg + '。请手动关闭并重新打开应用。'
      document.body.appendChild(div)
      setTimeout(() => div.remove(), 5000)
    }
  }
}

// @ts-ignore
async function handleQuit() {
  if (!confirm('确认关闭应用吗？')) return
  try {
    await api.quitApp()
  } catch (e) { /* 进程已退出，忽略 */ }
}
</script>

<style>
.app { display: flex; height: 100vh; }
.sidebar { width: 200px; background: var(--color-sidebar-bg); padding: 16px; border-right: 1px solid var(--color-border); overflow-y: auto; display: flex; flex-direction: column; }
.logo { display: flex; align-items: center; gap: 8px; font-size: 18px; margin: 0 0 20px 0; color: var(--color-primary); padding: 4px 12px; font-weight: 700; letter-spacing: 0.5px; }
.logo-img { width: 32px; height: 32px; border-radius: 6px; object-fit: cover; box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1); }
.logo-icon { font-size: 22px; }
.nav-icon { display: inline-block; width: 18px; text-align: center; margin-right: 4px; font-size: 14px; }

.nav-group { margin-bottom: 20px; }
.nav-group-title { font-size: 11px; color: var(--color-text-tertiary); margin-bottom: 6px; padding: 0 12px; letter-spacing: 1px; text-transform: uppercase; font-weight: 600; }
.bank-name { font-size: 13px; color: var(--color-text-secondary); margin-bottom: 6px; padding: 0 12px; word-break: break-all; font-weight: 500; }

.sidebar nav { display: flex; flex-direction: column; gap: 2px; }
.sidebar nav a { text-decoration: none; color: var(--color-text); padding: 7px 12px; border-radius: var(--radius-md); font-size: 14px; transition: background 0.12s; }
.sidebar nav a:hover { background: var(--color-border-light); }
.sidebar nav a.router-link-active { background: var(--color-primary); color: #fff; }

.content { flex: 1; overflow: auto; padding: 24px; }

.sidebar-footer {
  margin-top: auto;
  padding: 12px 12px 4px;
  border-top: 1px solid var(--color-border-light);
}
.donate-btn {
  width: 100%;
  display: flex; align-items: center; justify-content: center; gap: 6px;
  padding: 8px 12px;
  background: var(--color-card);
  border: 1px solid var(--color-border-light);
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--color-text-secondary);
  font-size: 13px;
  transition: all 0.15s;
}
.donate-btn:hover {
  background: var(--color-warning-light);
  border-color: var(--color-warning);
  color: var(--color-warning);
}
.coffee { font-size: 16px; }

.app-actions {
  display: flex;
  gap: 6px;
  margin-top: 8px;
}
.app-action-btn {
  flex: 1;
  padding: 6px 0;
  border: 1px solid var(--color-border-light);
  background: var(--color-card);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 16px;
  color: var(--color-text-secondary);
  transition: all 0.15s;
}
.app-action-btn:hover {
  background: var(--color-primary-light);
  border-color: var(--color-primary);
  color: var(--color-primary);
}
.app-action-btn.danger {
  color: var(--color-text-secondary);
}
.app-action-btn.danger:hover {
  background: #e53935;
  border-color: #e53935;
  color: #fff;
}

</style>
