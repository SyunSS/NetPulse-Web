import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import type { UserInfo } from '@/api/auth'

const STORAGE_KEY = 'netpulse-auth'

export const useAuthStore = defineStore('auth', () => {
  // 从 localStorage 恢复
  const stored = localStorage.getItem(STORAGE_KEY)
  const parsed = stored ? JSON.parse(stored) : null

  const token = ref<string>(parsed?.token || '')
  const user = ref<UserInfo | null>(parsed?.user || null)

  const isLoggedIn = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.role === 'admin')

  // 监听变化自动持久化
  watch([token, user], () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ token: token.value, user: user.value }),
    )
  })

  function setAuth(t: string, u: UserInfo) {
    token.value = t
    user.value = u
  }

  function logout() {
    token.value = ''
    user.value = null
  }

  return { token, user, isLoggedIn, isAdmin, setAuth, logout }
})
