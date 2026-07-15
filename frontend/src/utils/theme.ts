import { ref, watchEffect } from 'vue'

const isDark = ref(false)

export function useDark() {
  // 从 localStorage 读取主题偏好
  const stored = localStorage.getItem('netpulse-theme')
  if (stored === 'dark') {
    isDark.value = true
  } else if (stored === 'light') {
    isDark.value = false
  } else {
    // 跟随系统
    isDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
  }

  watchEffect(() => {
    localStorage.setItem('netpulse-theme', isDark.value ? 'dark' : 'light')
    document.documentElement.classList.toggle('dark', isDark.value)
  })

  function toggleDark() {
    isDark.value = !isDark.value
  }

  return { isDark, toggleDark }
}
