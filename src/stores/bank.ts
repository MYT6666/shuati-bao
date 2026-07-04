import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, QuizBank } from '../utils/api'

export const useBankStore = defineStore('bank', () => {
  const banks = ref<QuizBank[]>([])
  const loading = ref(false)

  async function load() {
    loading.value = true
    try {
      banks.value = await api.listBanks()
    } finally {
      loading.value = false
    }
  }
  async function create(name: string, description: string | null) {
    const b = await api.createBank({ name, description })
    banks.value.unshift(b)
    return b
  }
  async function remove(id: number) {
    await api.deleteBank(id)
    banks.value = banks.value.filter(b => b.id !== id)
  }
  return { banks, loading, load, create, remove }
})
