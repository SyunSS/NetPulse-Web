<script setup lang="ts">
import type { UserInfo } from '@/api/auth'

defineProps<{
  isDark: boolean
  user: UserInfo | null
}>()

const emit = defineEmits<{
  'toggle-dark': []
  logout: []
}>()
</script>

<template>
  <div class="app-header">
    <div class="header-left">
      <n-gradient-text type="info" :size="22" class="logo">
        NetPulse Web
      </n-gradient-text>
    </div>
    <div class="header-right">
      <n-button
        quaternary
        circle
        @click="emit('toggle-dark')"
      >
        <template #icon>
          <n-icon>
            {{ isDark ? '☀️' : '🌙' }}
          </n-icon>
        </template>
      </n-button>
      <n-dropdown
        trigger="click"
        :options="[
          { label: '退出登录', key: 'logout' },
        ]"
        @select="emit('logout')"
      >
        <n-button quaternary>
          {{ user?.username || '用户' }}
        </n-button>
      </n-dropdown>
    </div>
  </div>
</template>

<style scoped>
.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  height: 60px;
}
.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}
.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.logo {
  font-weight: 700;
  cursor: pointer;
}
</style>
