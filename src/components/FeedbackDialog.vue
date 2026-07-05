<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-backdrop" @click.self="close">
      <div class="modal feedback-dialog" role="dialog" aria-modal="true" aria-labelledby="fb-title">
        <header class="modal-head">
          <h3 id="fb-title">💌 意见反馈</h3>
          <button class="icon-btn" @click="close" aria-label="关闭">✕</button>
        </header>

        <div class="modal-body">
          <p class="intro">
            遇到 Bug？想要新功能？有其他建议？<br />
            告诉我们，反馈会直接发到作者的 QQ 邮箱。
          </p>

          <div class="field">
            <label>反馈类型</label>
            <div class="cat-row">
              <button
                v-for="c in categories"
                :key="c.key"
                class="cat-btn"
                :class="{ active: category === c.key }"
                @click="category = c.key"
                type="button"
              >
                {{ c.icon }} {{ c.label }}
              </button>
            </div>
          </div>

          <div class="field">
            <label>标题 <span class="required">*</span></label>
            <input
              v-model="title"
              maxlength="80"
              :placeholder="titlePlaceholder"
              class="text-input"
            />
            <span class="char-count">{{ title.length }} / 80</span>
          </div>

          <div class="field">
            <label>详细描述 <span class="required">*</span></label>
            <textarea
              v-model="description"
              rows="6"
              maxlength="2000"
              :placeholder="descPlaceholder"
              class="text-input"
            ></textarea>
            <span class="char-count">{{ description.length }} / 2000</span>
          </div>

          <div class="field">
            <label>联系方式（可选）</label>
            <input
              v-model="contact"
              maxlength="60"
              placeholder="邮箱 / QQ / 微信 / GitHub ID"
              class="text-input"
            />
            <p class="hint">方便作者回复你；不填则不回复</p>
          </div>

          <div class="field">
            <label class="checkbox-label">
              <input type="checkbox" v-model="includeLogs" />
              <span>附加最近 100 行运行日志（强烈建议，便于排查 Bug）</span>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" v-model="includeSystemInfo" checked disabled />
              <span>附加系统信息（OS / 版本号）</span>
            </label>
          </div>

          <details v-if="previewMarkdown" class="preview-box">
            <summary>👁 预览反馈内容（Markdown 格式）</summary>
            <pre class="md-preview">{{ previewMarkdown }}</pre>
          </details>

          <div v-if="resultMsg" :class="['result', resultOk ? 'ok' : 'fail']">
            {{ resultMsg }}
          </div>
        </div>

        <footer class="modal-foot">
          <button class="btn-secondary" @click="close" :disabled="submitting">取消</button>
          <button class="btn-primary" @click="sendToQQMail" :disabled="!canSubmit || submitting">
            � 发送到 QQ 邮箱
          </button>
          <button class="btn-secondary" @click="copyToClipboard" :disabled="!canSubmit || submitting">
            � 复制到剪贴板
          </button>
          <button class="btn-secondary" @click="saveLocal" :disabled="!canSubmit || submitting">
            💾 保存到本地
          </button>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { toastSuccess, toastError } from '../utils/toast'

interface Props {
  visible: boolean
}
const props = defineProps<Props>()
const emit = defineEmits<{ (e: 'close'): void }>()

const categories = [
  { key: 'bug', label: 'Bug 报告', icon: '🐛' },
  { key: 'feature', label: '功能建议', icon: '💡' },
  { key: 'question', label: '使用问题', icon: '❓' },
  { key: 'other', label: '其他', icon: '📝' },
]

const category = ref('bug')
const title = ref('')
const description = ref('')
const contact = ref('')
const includeLogs = ref(true)
const includeSystemInfo = ref(true)
const submitting = ref(false)
const resultMsg = ref('')
const resultOk = ref(false)
const previewMarkdown = ref('')

const titlePlaceholder = computed(() => {
  const map: Record<string, string> = {
    bug: '一句话描述 bug，如：导入 PDF 时闪退',
    feature: '希望增加什么功能？',
    question: '不知道怎么操作...',
    other: '其他反馈',
  }
  return map[category.value] || ''
})

const descPlaceholder = computed(() => {
  const map: Record<string, string> = {
    bug: '详细描述：\n1. 操作步骤\n2. 预期结果\n3. 实际结果\n4. 出现频率（每次/偶尔）',
    feature: '希望解决什么问题？\n希望怎么实现？',
    question: '详细说明你卡在哪里',
    other: '想说的话',
  }
  return map[category.value] || ''
})

const canSubmit = computed(() => {
  return title.value.trim().length > 0 && description.value.trim().length > 0
})

watch(
  () => props.visible,
  (v) => {
    if (v) {
      // 打开时重置
      resultMsg.value = ''
      resultOk.value = false
      previewMarkdown.value = ''
    }
  }
)

async function buildPayload() {
  if (!canSubmit.value) {
    toastError('请填写标题和详细描述')
    return null
  }
  submitting.value = true
  try {
    const payload = await invoke<{ markdown: string; system_info: string; recent_logs: string }>(
      'build_feedback',
      {
        category: category.value,
        title: title.value.trim(),
        description: description.value.trim(),
        contact: contact.value.trim(),
      }
    )
    let md = payload.markdown
    if (!includeLogs.value) {
      // 移除日志部分
      md = md.replace(/### 最近日志[\s\S]*$/, '')
    }
    return md
  } catch (e) {
    toastError('构造反馈失败：' + (e instanceof Error ? e.message : String(e)))
    return null
  } finally {
    submitting.value = false
  }
}

async function copyToClipboard() {
  const md = await buildPayload()
  if (!md) return
  try {
    await navigator.clipboard.writeText(md)
    toastSuccess('已复制到剪贴板，去 GitHub 粘贴即可')
    resultOk.value = true
    resultMsg.value = '✓ 已复制 Markdown 到剪贴板'
  } catch (e) {
    // 降级：使用 execCommand
    try {
      const ta = document.createElement('textarea')
      ta.value = md
      ta.style.position = 'fixed'
      ta.style.left = '-9999px'
      document.body.appendChild(ta)
      ta.select()
      document.execCommand('copy')
      document.body.removeChild(ta)
      toastSuccess('已复制到剪贴板')
      resultOk.value = true
      resultMsg.value = '✓ 已复制 Markdown 到剪贴板'
    } catch (e2) {
      toastError('复制失败：' + (e instanceof Error ? e.message : String(e)))
    }
  }
}

async function sendToQQMail() {
  const md = await buildPayload()
  if (!md) return
  // mailto URL 长度限制：约 2048 字符（IE/Outlook），Chrome/Edge/QQ 邮件客户端约 8192
  // 超长内容会被截断，所以同时复制到剪贴板
  const FEEDBACK_EMAIL = '2943663274@qq.com'
  const categoryLabel = {
    bug: '[Bug]',
    feature: '[建议]',
    question: '[问题]',
    other: '[其他]',
  }[category.value] || '[' + category.value + ']'
  const subject = `刷题宝意见反馈 ${categoryLabel} ${title.value.trim()}`

  let body = md
  // mailto URL 有长度限制，超长会失败；所以把完整 md 复制到剪贴板，body 里放说明
  if (md.length > 1500) {
    body =
      `用户反馈内容较长，已自动复制到剪贴板。\n\n` +
      `请在邮件编辑页中按 Ctrl+V 粘贴完整内容。\n\n` +
      `----------\n` +
      md.slice(0, 800) +
      `\n\n...(内容过长已截断，完整内容请从剪贴板粘贴)...`
    try {
      await navigator.clipboard.writeText(md)
    } catch {}
  }

  const mailtoUrl =
    `mailto:${FEEDBACK_EMAIL}` +
    `?subject=${encodeURIComponent(subject)}` +
    `&body=${encodeURIComponent(body)}`

  // 用 window.location 触发 mailto 协议（最稳的跨平台方式）
  try {
    window.location.href = mailtoUrl
    resultOk.value = true
    resultMsg.value = '✓ 已唤起系统默认邮件客户端（QQ 邮箱），请在邮件编辑页点发送'
  } catch (e) {
    // 降级：复制到剪贴板 + 提示
    try {
      await navigator.clipboard.writeText(
        `收件人: ${FEEDBACK_EMAIL}\n主题: ${subject}\n\n${md}`
      )
    } catch {}
    toastError('打开邮件客户端失败，反馈内容已复制到剪贴板，请手动粘贴发送')
  }
}

async function saveLocal() {
  const md = await buildPayload()
  if (!md) return
  try {
    const path = await invoke<string>('save_feedback_local', { content: md })
    toastSuccess('已保存到：' + path)
    resultOk.value = true
    resultMsg.value = '✓ 已保存到本地，可通过"打开反馈目录"查看'
  } catch (e) {
    toastError('保存失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

function close() {
  emit('close')
}
</script>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(2px);
}
.modal {
  background: var(--bg, #fff);
  color: var(--text, #222);
  border-radius: 12px;
  width: 90%;
  max-width: 640px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}
.feedback-dialog {
  max-width: 720px;
}
.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}
.modal-head h3 {
  margin: 0;
  font-size: 18px;
}
.icon-btn {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  color: var(--text-muted, #888);
  padding: 4px 8px;
  border-radius: 6px;
}
.icon-btn:hover {
  background: rgba(0, 0, 0, 0.05);
}
.modal-body {
  padding: 20px;
  overflow-y: auto;
  flex: 1;
}
.modal-foot {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  padding: 12px 20px;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
  flex-wrap: wrap;
}
.intro {
  color: var(--text-muted, #666);
  font-size: 13px;
  margin: 0 0 16px;
  line-height: 1.5;
}
.field {
  margin-bottom: 14px;
}
.field label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 6px;
}
.required {
  color: #ef4444;
}
.text-input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  border-radius: 6px;
  font-size: 13px;
  font-family: inherit;
  background: var(--input-bg, #fff);
  color: var(--text, #222);
  box-sizing: border-box;
  resize: vertical;
}
.text-input:focus {
  outline: none;
  border-color: #2563eb;
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.15);
}
textarea.text-input {
  min-height: 100px;
  font-family: 'Consolas', 'Monaco', monospace;
}
.char-count {
  display: block;
  text-align: right;
  font-size: 11px;
  color: var(--text-muted, #999);
  margin-top: 2px;
}
.cat-row {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.cat-btn {
  flex: 1;
  min-width: 110px;
  padding: 8px 10px;
  border: 1px solid rgba(0, 0, 0, 0.15);
  border-radius: 6px;
  background: var(--input-bg, #fff);
  color: var(--text, #222);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.15s;
}
.cat-btn:hover {
  border-color: #2563eb;
}
.cat-btn.active {
  background: #2563eb;
  color: #fff;
  border-color: #2563eb;
}
.checkbox-label {
  display: flex !important;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: normal !important;
  margin-bottom: 4px;
  cursor: pointer;
}
.checkbox-label input {
  cursor: pointer;
}
.hint {
  font-size: 11px;
  color: var(--text-muted, #999);
  margin: 4px 0 0;
}
.preview-box {
  margin-top: 12px;
  background: rgba(0, 0, 0, 0.03);
  padding: 10px;
  border-radius: 6px;
}
.preview-box summary {
  cursor: pointer;
  font-size: 13px;
  color: var(--text-muted, #666);
  margin-bottom: 8px;
}
.md-preview {
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 11px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 200px;
  overflow-y: auto;
  background: var(--input-bg, #fff);
  padding: 8px;
  border-radius: 4px;
  margin: 0;
  border: 1px solid rgba(0, 0, 0, 0.08);
}
.result {
  margin-top: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
}
.result.ok {
  background: rgba(34, 197, 94, 0.1);
  color: #16a34a;
  border: 1px solid rgba(34, 197, 94, 0.3);
}
.result.fail {
  background: rgba(239, 68, 68, 0.1);
  color: #dc2626;
  border: 1px solid rgba(239, 68, 68, 0.3);
}
.btn-primary {
  background: #2563eb;
  color: #fff;
  border: none;
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
}
.btn-primary:hover:not(:disabled) {
  background: #1d4ed8;
}
.btn-primary:disabled {
  background: #93c5fd;
  cursor: not-allowed;
}
.btn-secondary {
  background: transparent;
  color: var(--text, #222);
  border: 1px solid rgba(0, 0, 0, 0.15);
  padding: 8px 14px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}
.btn-secondary:hover:not(:disabled) {
  background: rgba(0, 0, 0, 0.05);
}
.btn-secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
