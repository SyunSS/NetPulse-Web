<script setup lang="ts">
import { useDark } from '@/utils/theme'
import { useAuthStore } from '@/stores/auth'
import { useRouter } from 'vue-router'
import AppHeader from '@/components/AppHeader.vue'

const { isDark, toggleDark } = useDark()
const authStore = useAuthStore()
const router = useRouter()

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<template>
  <n-layout class="app-layout">
    <n-layout-header bordered>
      <AppHeader
        :is-dark="isDark"
        :user="authStore.user"
        @toggle-dark="toggleDark"
        @logout="handleLogout"
      />
    </n-layout-header>
    <n-layout-content class="app-content">
      <router-view />
    </n-layout-content>
  </n-layout>
</template>

<style scoped>
.app-layout {
  min-height: 100vh;
}
.app-content {
  padding: 24px;
  max-width: 1400px;
  margin: 0 auto;
  width: 100%;
}
</style>
