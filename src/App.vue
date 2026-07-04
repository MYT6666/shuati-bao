<template>
  <div class="app">
    <aside class="sidebar">
      <h1 class="logo">刷题宝</h1>

      <div class="nav-group">
        <div class="nav-group-title">全局</div>
        <nav>
          <RouterLink to="/">题库</RouterLink>
          <RouterLink to="/settings">设置</RouterLink>
        </nav>
      </div>

      <div v-if="currentBank" class="nav-group">
        <div class="nav-group-title">当前题库</div>
        <div class="bank-name">{{ currentBank.name }}</div>
        <nav>
          <RouterLink :to="`/practice/${currentBank.id}`">练习</RouterLink>
          <RouterLink :to="`/wrong/${currentBank.id}`">错题本</RouterLink>
          <RouterLink :to="`/favorites/${currentBank.id}`">收藏</RouterLink>
          <RouterLink :to="`/stats/${currentBank.id}`">统计</RouterLink>
        </nav>
      </div>

      <div class="sidebar-footer">
        <button class="donate-btn" @click="showDonate = true" title="支持作者">
          <span class="coffee">💝</span>
          <span class="donate-text">支持作者</span>
        </button>
      </div>
    </aside>
    <main class="content"><RouterView /></main>

    <DonateDialog :visible="showDonate" @close="showDonate = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { RouterView, RouterLink, useRoute } from 'vue-router'
import { useBankStore } from './stores/bank'
import DonateDialog from './components/DonateDialog.vue'

const route = useRoute()
const bankStore = useBankStore()
const currentBank = ref<{ id: number; name: string } | null>(null)
const showDonate = ref(false)

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
</script>

<style>
.app { display: flex; height: 100vh; }
.sidebar { width: 200px; background: var(--color-sidebar-bg); padding: 16px; border-right: 1px solid var(--color-border); overflow-y: auto; display: flex; flex-direction: column; }
.logo { font-size: 20px; margin: 0 0 20px 0; color: var(--color-primary); padding: 0 12px; }

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

</style>
