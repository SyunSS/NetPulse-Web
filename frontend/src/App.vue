<script setup lang="ts">
import { darkTheme, zhCN, dateZhCN } from 'naive-ui'
import { useDark } from '@/utils/theme'
import { useAuthStore } from '@/stores/auth'

const { isDark } = useDark()
const authStore = useAuthStore()

// 如果已登录，使用 NaiveUI 的暗色主题
</script>

<template>
  <n-config-provider
    :theme="isDark ? darkTheme : undefined"
    :locale="zhCN"
    :date-locale="dateZhCN"
  >
    <n-message-provider>
      <router-view />
    </n-message-provider>
  </n-config-provider>
</template>
