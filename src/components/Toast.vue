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
}

const toasts = ref<ToastItem[]>([])
let nextId = 1
let listener: ((e: Event) => void) | null = null

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

function show(type: ToastType, message: string, duration = 2400) {
  const id = nextId++
  toasts.value.push({ id, type, message, duration })
  if (duration > 0) {
    setTimeout(() => remove(id), duration)
  }
}

defineExpose({ show, remove })

onMounted(() => {
  listener = (e: Event) => {
    const ce = e as CustomEvent
    if (ce.detail && typeof ce.detail.type === 'string') {
      show(ce.detail.type, ce.detail.message, ce.detail.duration ?? 2400)
    }
  }
  window.addEventListener('app-toast', listener)
})
onBeforeUnmount(() => {
  if (listener) window.removeEventListener('app-toast', listener)
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
  gap: 10px;
  min-width: 240px;
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
.toast-msg { flex: 1; word-break: break-word; }
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

.toast-enter-from { opacity: 0; transform: translateX(20px); }
.toast-enter-active { transition: all 0.25s ease; }
.toast-leave-to { opacity: 0; transform: translateX(20px); }
.toast-leave-active { transition: all 0.2s ease; }
</style>
