// 全局 Toast 工具：通过 CustomEvent 解耦，直接调用即可
import type { ToastType } from '../components/Toast.vue'

export function toast(type: ToastType, message: string, duration = 2400) {
  window.dispatchEvent(new CustomEvent('app-toast', { detail: { type, message, duration } }))
}

export const toastSuccess = (msg: string, duration = 2400) => toast('success', msg, duration)
export const toastError = (msg: string, duration = 3500) => toast('error', msg, duration)
export const toastInfo = (msg: string, duration = 2400) => toast('info', msg, duration)
export const toastWarning = (msg: string, duration = 3000) => toast('warning', msg, duration)
