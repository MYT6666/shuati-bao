<template>
  <Teleport to="body">
    <transition name="fade">
      <div v-if="visible" class="donate-mask" @click.self="close">
        <div class="donate-card">
          <button class="close-btn" @click="close" aria-label="关闭">×</button>

          <div class="donate-header">
            <div class="heart">💝</div>
            <h2>支持作者</h2>
          </div>

          <div class="donate-message">
            <p>嗨，谢谢你用到了这里。</p>
            <p>
              刷题宝是一个人在业余时间断断续续做出来的小工具，
              没有团队，没有融资，代码和数据都老老实实放在你自己的电脑里。
            </p>
            <p>
              每次看到有人因为这个小软件少背了几页书、多刷了几道题、考试顺手了一点，
              就是这件事继续下去最大的理由。
            </p>
            <p>
              如果觉得好用，可以打赏支持一下，金额随意，心意到了就好。
              <br>如果暂时不方便，也完全没关系，
              把它用熟、推荐给同样在备考的朋友，对我来说同样重要。
            </p>
            <p class="signature">—— 一个还在熬夜写代码的开发者</p>
          </div>

          <div class="qr-area">
            <div class="qr-item">
              <div class="qr-img wechat">
                <img v-if="wechatImg" :src="wechatImg" alt="微信收款码" />
                <div v-else class="qr-placeholder">
                  <span>微信收款码</span>
                  <small>请放置图片到<br>src/assets/wechat-qr.png</small>
                </div>
              </div>
              <div class="qr-label">
                <span class="ic wechat-ic">💚</span> 微信
              </div>
            </div>
            <div class="qr-item">
              <div class="qr-img alipay">
                <img v-if="alipayImg" :src="alipayImg" alt="支付宝收款码" />
                <div v-else class="qr-placeholder">
                  <span>支付宝收款码</span>
                  <small>请放置图片到<br>src/assets/alipay-qr.png</small>
                </div>
              </div>
              <div class="qr-label">
                <span class="ic alipay-ic">💙</span> 支付宝
              </div>
            </div>
          </div>

          <p class="thanks">无论是否打赏，谢谢你一直用到这里 ❤️</p>
        </div>
      </div>
    </transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'

defineProps<{ visible: boolean }>()
const emit = defineEmits<{ (e: 'close'): void }>()

// 直接按实际文件名导入（glob 在某些 vite 版本可能不稳定）
const wechatImg = ref<string | null>(null)
const alipayImg = ref<string | null>(null)

async function loadImages() {
  // 微信：wechat-qr.png
  try { wechatImg.value = (await import('../assets/wechat-qr.png?url')).default } catch {}
  // 支付宝：alipay-qr.jpg
  try { alipayImg.value = (await import('../assets/alipay-qr.jpg?url')).default } catch {}
}

function close() { emit('close') }
function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') close()
}

onMounted(() => {
  loadImages()
  window.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))
</script>

<style scoped>
.donate-mask {
  position: fixed; inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex; align-items: center; justify-content: center;
  z-index: 9999;
  backdrop-filter: blur(2px);
}
.donate-card {
  position: relative;
  background: var(--color-card);
  border-radius: 16px;
  padding: 28px 32px 24px;
  width: 460px;
  max-width: 92vw;
  max-height: 92vh;
  overflow-y: auto;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.3);
  color: var(--color-text);
}
.close-btn {
  position: absolute; top: 12px; right: 16px;
  background: none; border: none; cursor: pointer;
  font-size: 24px; color: var(--color-text-tertiary);
  line-height: 1; padding: 4px;
}
.close-btn:hover { color: var(--color-text); }

.donate-header { text-align: center; margin-bottom: 16px; }
.heart { font-size: 40px; margin-bottom: 4px; }
.donate-header h2 {
  margin: 0;
  font-size: 20px;
  color: var(--color-text);
  font-weight: 600;
}

.donate-message {
  font-size: 14px;
  line-height: 1.75;
  color: var(--color-text-secondary);
  margin-bottom: 20px;
}
.donate-message p { margin: 0 0 10px; }
.donate-message p:first-child {
  font-size: 15px;
  color: var(--color-text);
  font-weight: 500;
}
.signature {
  text-align: right;
  font-size: 13px;
  color: var(--color-text-tertiary);
  margin-top: 4px !important;
}

.qr-area {
  display: flex;
  justify-content: center;
  gap: 32px;
  margin: 20px 0;
}
.qr-item { text-align: center; }
.qr-img {
  width: 160px; height: 160px;
  border-radius: 12px;
  border: 1px solid var(--color-border);
  overflow: hidden;
  background: #fff;
  display: flex; align-items: center; justify-content: center;
}
.qr-img img { width: 100%; height: 100%; object-fit: contain; }
.qr-placeholder {
  display: flex; flex-direction: column; align-items: center;
  color: #999; font-size: 13px; text-align: center;
  padding: 12px;
}
.qr-placeholder span { font-weight: 500; color: #666; margin-bottom: 6px; }
.qr-placeholder small { font-size: 11px; line-height: 1.4; color: #aaa; }
.qr-label {
  margin-top: 8px;
  font-size: 14px;
  font-weight: 500;
  color: var(--color-text);
}
.ic { margin-right: 4px; }

.thanks {
  text-align: center;
  font-size: 13px;
  color: var(--color-text-tertiary);
  margin: 16px 0 0;
}

.fade-enter-active, .fade-leave-active { transition: opacity 0.2s; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
