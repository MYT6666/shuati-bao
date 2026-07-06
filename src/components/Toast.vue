<template>
  <Teleport to="body">
    <div class="toast-container">
      <transition-group name="toast">
        <div
          v-for="t in toasts"
          :key="t.id"
          class="toast"
          :class="['toast-' + t.type]"
        >
          <span class="toast-icon">{{ iconFor(t.type) }}</span>
          <span class="toast-msg">{{ t.message }}</span>
          <button v-if="t.duration === 0" class="toast-close" @click="remove(t.id)">×</button>
          <div v-if="typeof t.progress === 'number'" class="toast-progress-bar">
            <div class="toast-progress-fill" :style="{ width: t.progress + '%' }"></div>
          </div>
        </div>
      </transition-group>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'

export type ToastType = 'success' | 'error' | 'info' | 'warning'

interface ToastItem {
  id: number
  type: ToastType
  message: string
  duration: number
  progress?: number
}

const toasts = ref<ToastItem[]>([])
let nextId = 1
let showListener: ((e: Event) => void) | null = null
let updateListener: ((e: Event) => void) | null = null
let removeListener: ((e: Event) => void) | null = null

function iconFor(t: ToastType): string {
  switch (t) {
    case 'success': return '✓'
    case 'error': return '✕'
    case 'warning': return '⚠'
    default: return 'ℹ'
  }
}

function remove(id: number) {
  toasts.value = toasts.value.filter(t => t.id !== id)
}

function show(type: ToastType, message: string, duration = 2400, progress?: number, customId?: number): number {
  const id = customId ?? nextId++
  toasts.value = toasts.value.filter(t => t.id !== id)
  toasts.value.push({ id, type, message, duration, progress })
  if (duration > 0) {
    setTimeout(() => remove(id), duration)
  }
  return id
}

function update(id: number, opts: { message?: string; progress?: number; type?: ToastType }) {
  const t = toasts.value.find(t => t.id === id)
  if (!t) return
  if (opts.message !== undefined) t.message = opts.message
  if (opts.progress !== undefined) t.progress = opts.progress
  if (opts.type !== undefined) t.type = opts.type
}

defineExpose({ show, remove, update })

onMounted(() => {
  showListener = (e: Event) => {
    const ce = e as CustomEvent
    if (ce.detail && typeof ce.detail.type === 'string') {
      show(ce.detail.type, ce.detail.message, ce.detail.duration ?? 2400, ce.detail.progress, ce.detail.id)
    }
  }
  updateListener = (e: Event) => {
    const ce = e as CustomEvent
    if (ce.detail && typeof ce.detail.id === 'number') {
      update(ce.detail.id, {
        message: ce.detail.message,
        progress: ce.detail.progress,
        type: ce.detail.type,
      })
    }
  }
  removeListener = (e: Event) => {
    const ce = e as CustomEvent
    if (ce.detail && typeof ce.detail.id === 'number') {
      remove(ce.detail.id)
    }
  }
  window.addEventListener('app-toast', showListener)
  window.addEventListener('app-toast-update', updateListener)
  window.addEventListener('app-toast-remove', removeListener)
})
onBeforeUnmount(() => {
  if (showListener) window.removeEventListener('app-toast', showListener)
  if (updateListener) window.removeEventListener('app-toast-update', updateListener)
  if (removeListener) window.removeEventListener('app-toast-remove', removeListener)
})
</script>

<style scoped>
.toast-container {
  position: fixed;
  top: 24px;
  right: 24px;
  z-index: 9999;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
}
.toast {
  pointer-events: auto;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  min-width: 280px;
  max-width: 420px;
  padding: 10px 16px;
  background: var(--color-card, #fff);
  border-radius: var(--radius-md, 6px);
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.12);
  border-left: 4px solid #6a8dff;
  font-size: 14px;
  color: var(--color-text, #0f0f0f);
  line-height: 1.5;
}
.toast-success { border-left-color: #16a34a; }
.toast-success .toast-icon { color: #16a34a; }
.toast-error { border-left-color: #dc2626; }
.toast-error .toast-icon { color: #dc2626; }
.toast-warning { border-left-color: #f59e0b; }
.toast-warning .toast-icon { color: #f59e0b; }
.toast-info { border-left-color: #6a8dff; }
.toast-info .toast-icon { color: #6a8dff; }
.toast-icon {
  font-weight: 700;
  font-size: 16px;
  flex-shrink: 0;
}
.toast-msg { flex: 1; word-break: break-word; min-width: 160px; }
.toast-close {
  background: none;
  border: none;
  color: var(--color-text-tertiary, #999);
  cursor: pointer;
  font-size: 18px;
  line-height: 1;
  padding: 0 4px;
}
.toast-close:hover { color: var(--color-text, #0f0f0f); }

.toast-progress-bar {
  width: 100%;
  height: 4px;
  background: var(--color-border-light, #eee);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 2px;
}
.toast-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #6a8dff, #8b5cf6);
  border-radius: 2px;
  transition: width 0.3s ease;
}
.toast-success .toast-progress-fill { background: linear-gradient(90deg, #16a34a, #22c55e); }
.toast-error .toast-progress-fill { background: linear-gradient(90deg, #dc2626, #ef4444); }
.toast-warning .toast-progress-fill { background: linear-gradient(90deg, #f59e0b, #fbbf24); }

.toast-enter-from { opacity: 0; transform: translateX(20px); }
.toast-enter-active { transition: all 0.25s ease; }
.toast-leave-to { opacity: 0; transform: translateX(20px); }
.toast-leave-active { transition: all 0.2s ease; }
</style>
