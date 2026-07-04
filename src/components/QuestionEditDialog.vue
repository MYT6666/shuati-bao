<template>
  <Teleport to="body">
    <transition name="fade">
      <div v-if="visible" class="edit-mask" @click.self="close">
        <div class="edit-card">
          <button class="close-btn" @click="close" aria-label="关闭">×</button>
          <h2>编辑题目</h2>

          <div class="form-row">
            <label>类型</label>
            <select v-model="form.type">
              <option value="single">单选</option>
              <option value="multi">多选</option>
              <option value="judge">判断</option>
              <option value="blank">填空</option>
              <option value="qa">问答</option>
            </select>
          </div>

          <div class="form-row">
            <label>题干</label>
            <textarea v-model="form.stem" rows="4" placeholder="题干内容"></textarea>
          </div>

          <!-- 选项（仅 single/multi） -->
          <div v-if="form.type === 'single' || form.type === 'multi'" class="form-row">
            <label>选项 <button class="mini-btn" @click="addOption">+ 添加</button></label>
            <div v-for="(_opt, i) in form.options" :key="i" class="option-edit">
              <span class="opt-letter">{{ letter(i) }}</span>
              <input v-model="form.options[i]" :placeholder="`选项 ${letter(i)}`" />
              <button v-if="form.options.length > 2" class="mini-btn danger" @click="removeOption(i)">✕</button>
            </div>
          </div>

          <!-- 答案 -->
          <div class="form-row">
            <label>答案</label>
            <!-- 单选 -->
            <select v-if="form.type === 'single'" v-model="form.answer">
              <option v-for="(opt, i) in form.options" :key="i" :value="letter(i)">{{ letter(i) }}{{ opt ? ' - ' + opt.slice(0, 20) : '' }}</option>
            </select>
            <!-- 多选 -->
            <div v-else-if="form.type === 'multi'" class="multi-answer">
              <label v-for="(opt, i) in form.options" :key="i" class="checkbox-label">
                <input type="checkbox" :value="letter(i)" v-model="multiAnswer" />
                <span>{{ letter(i) }}{{ opt ? ' - ' + opt.slice(0, 20) : '' }}</span>
              </label>
            </div>
            <!-- 判断 -->
            <div v-else-if="form.type === 'judge'" class="judge-answer">
              <label class="radio-label"><input type="radio" value="true" v-model="form.answer" /> 正确</label>
              <label class="radio-label"><input type="radio" value="false" v-model="form.answer" /> 错误</label>
            </div>
            <!-- 填空/问答 -->
            <textarea v-else v-model="form.answer" rows="2" placeholder="答案文本"></textarea>
          </div>

          <div class="form-row">
            <label>解析（可选）</label>
            <textarea v-model="form.analysis" rows="3" placeholder="解题思路"></textarea>
          </div>

          <div class="form-actions">
            <button class="cancel-btn" @click="close">取消</button>
            <button class="save-btn" :disabled="saving" @click="save">{{ saving ? '保存中...' : '保存' }}</button>
          </div>
        </div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { api, Question } from '../utils/api'
import { toastError } from '../utils/toast'

const props = defineProps<{ visible: boolean; question: Question | null }>()
const emit = defineEmits<{ (e: 'close'): void; (e: 'saved', q: Question): void }>()

const form = ref<{ type: string; stem: string; options: string[]; answer: string; analysis: string }>({
  type: 'single', stem: '', options: [], answer: '', analysis: ''
})
const multiAnswer = ref<string[]>([])
const saving = ref(false)

// 当 question 变化时填充表单
watch(() => props.question, (q) => {
  if (!q) return
  // 解析 options
  let options: string[] = []
  if (q.options) {
    try { options = JSON.parse(q.options) } catch { options = [] }
  }
  // 解析 answer
  let answer = q.answer || ''
  if (q.type === 'multi' && answer) {
    try { multiAnswer.value = JSON.parse(answer) } catch { multiAnswer.value = answer.split('') }
  } else {
    multiAnswer.value = []
  }
  form.value = {
    type: q.type,
    stem: q.stem,
    options,
    answer,
    analysis: q.analysis || ''
  }
}, { immediate: true })

// 多选答案变化时同步到 form.answer
watch(multiAnswer, (vals) => {
  if (form.value.type === 'multi') {
    form.value.answer = JSON.stringify(vals.sort())
  }
}, { deep: true })

const letter = (i: number) => String.fromCharCode(65 + i)

function addOption() {
  form.value.options.push('')
}
function removeOption(i: number) {
  form.value.options.splice(i, 1)
}

function close() { emit('close') }

async function save() {
  if (!props.question) return
  saving.value = true
  try {
    const updated: Question = {
      ...props.question,
      type: form.value.type,
      stem: form.value.stem,
      options: (form.value.type === 'single' || form.value.type === 'multi') && form.value.options.length > 0
        ? JSON.stringify(form.value.options) : null,
      answer: form.value.answer || null,
      analysis: form.value.analysis || null,
    }
    await api.updateQuestion(updated)
    emit('saved', updated)
  } catch (e) {
    toastError('保存失败：' + (e instanceof Error ? e.message : String(e)))
  } finally {
    saving.value = false
  }
}
</script>

<style scoped>
.edit-mask {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex; align-items: center; justify-content: center;
  z-index: 9999;
}
.edit-card {
  position: relative;
  background: var(--color-card);
  border-radius: 12px;
  padding: 24px 28px;
  width: 600px;
  max-width: 92vw;
  max-height: 90vh;
  overflow-y: auto;
  box-shadow: 0 12px 40px rgba(0,0,0,0.3);
  color: var(--color-text);
}
.close-btn {
  position: absolute; top: 12px; right: 16px;
  background: none; border: none; cursor: pointer;
  font-size: 22px; color: var(--color-text-tertiary);
}
.close-btn:hover { color: var(--color-text); }
h2 { margin: 0 0 16px; font-size: 18px; }
.form-row { margin-bottom: 14px; }
.form-row label { display: block; font-size: 13px; color: var(--color-text-secondary); margin-bottom: 4px; font-weight: 500; }
textarea, input[type="text"], select {
  width: 100%; padding: 8px;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  background: var(--color-card);
  color: var(--color-text);
  font-family: inherit; font-size: 14px;
}
textarea { resize: vertical; }
.option-edit { display: flex; align-items: center; gap: 6px; margin-bottom: 6px; }
.opt-letter { font-weight: bold; width: 20px; color: var(--color-primary); }
.option-edit input { flex: 1; }
.mini-btn {
  padding: 2px 8px; border: 1px solid var(--color-border);
  border-radius: 4px; background: var(--color-card); cursor: pointer;
  font-size: 12px; color: var(--color-text-secondary);
}
.mini-btn.danger { color: var(--color-danger); border-color: var(--color-danger); }
.mini-btn.danger:hover { background: var(--color-danger-light); }
.multi-answer, .judge-answer { display: flex; flex-wrap: wrap; gap: 12px; }
.checkbox-label, .radio-label { display: flex; align-items: center; gap: 4px; cursor: pointer; font-size: 14px; }
.form-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
.cancel-btn {
  padding: 8px 16px; border: 1px solid var(--color-border);
  border-radius: 6px; background: var(--color-card); cursor: pointer;
  color: var(--color-text);
}
.save-btn {
  padding: 8px 20px; border: none;
  border-radius: 6px; background: var(--color-primary); color: #fff;
  cursor: pointer; font-weight: 500;
}
.save-btn:disabled { background: var(--color-text-tertiary); cursor: not-allowed; }
.fade-enter-active, .fade-leave-active { transition: opacity 0.2s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
