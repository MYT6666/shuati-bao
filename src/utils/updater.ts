// 自动更新工具
import { check as tauriCheck, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

// 重导出 check 函数，方便其他文件直接用
export const check = tauriCheck
export type { Update }

function toast(type: 'success' | 'error' | 'info' | 'warning', message: string, duration = 2400) {
  window.dispatchEvent(new CustomEvent('app-toast', { detail: { type, message, duration } }))
}

export interface UpdateCheckResult {
  hasUpdate: boolean
  currentVersion: string
  latestVersion?: string
  notes?: string
}

let checking = false
let lastCheckAt = 0
const CHECK_COOLDOWN = 1000 * 60 * 30 // 30 分钟内不重复检查（手动检查不受限）

/**
 * 检查是否有更新（带节流）
 * @param silent 静默模式：已是最新时是否仍 toast 提示
 */
export async function checkForUpdates(opts: { silent?: boolean } = {}): Promise<UpdateCheckResult | null> {
  if (checking) return null
  if (!opts.silent && Date.now() - lastCheckAt < CHECK_COOLDOWN) return null
  checking = true
  lastCheckAt = Date.now()
  try {
    const update = await check()
    if (!update) {
      if (!opts.silent) toast('success', '已是最新版本')
      return { hasUpdate: false, currentVersion: await getCurrentVersion() }
    }
    return {
      hasUpdate: true,
      currentVersion: await getCurrentVersion(),
      latestVersion: update.version,
      notes: typeof update.body === 'string' ? update.body : undefined
    }
  } catch (e) {
    console.error('检查更新失败：', e)
    if (!opts.silent) {
      const msg = e instanceof Error ? e.message : String(e)
      toast('error', '检查更新失败：' + msg)
    }
    return null
  } finally {
    checking = false
  }
}

/**
 * 弹窗提示用户更新，确认后下载安装并重启
 */
export async function promptAndApplyUpdate(update: Update): Promise<boolean> {
  const yes = window.confirm(
    `发现新版本 v${update.version}\n\n` +
    (update.body ? `更新内容：\n${update.body}\n\n` : '') +
    `是否立即下载并安装？`
  )
  if (!yes) return false

  toast('info', '正在下载更新...', 0)
  try {
    await update.downloadAndInstall((event) => {
      if (event.event === 'Progress') {
        console.log('下载进度：', event.data)
      } else if (event.event === 'Finished') {
        toast('success', '下载完成，准备重启...')
      }
    })
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    console.error('下载/安装更新失败：', e)
    toast('error', '更新失败：' + msg)
    return false
  }

  const relaunchNow = window.confirm('更新已下载完成，是否立即重启应用？')
  if (relaunchNow) {
    try {
      await relaunch()
    } catch (e) {
      toast('error', '重启失败：' + (e instanceof Error ? e.message : String(e)))
      return false
    }
  }
  return true
}

/**
 * 启动时后台静默检查，发现新版本时弹窗
 */
export async function autoCheckOnStartup(): Promise<void> {
  // 等待 3 秒再检查，避免干扰首屏
  await new Promise(r => setTimeout(r, 3000))
  const update = await check()
  if (update) {
    await promptAndApplyUpdate(update)
  }
}

async function getCurrentVersion(): Promise<string> {
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    return await getVersion()
  } catch {
    return '0.0.0'
  }
}
