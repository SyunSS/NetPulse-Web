<script setup lang="ts">
import { computed } from 'vue'
import { useDark } from '@/utils/theme'

const { isDark, toggleDark } = useDark()

const props = defineProps<{
  title: string
  subtitle?: string
}>()

const emit = defineEmits<{
  refresh: []
}>()
</script>

<template>
  <header class="app-header">
    <div class="header-left">
      <h1 class="header-title">{{ title }}</h1>
      <p v-if="subtitle" class="header-subtitle">{{ subtitle }}</p>
    </div>
    <div class="header-right">
      <button class="icon-btn" @click="emit('refresh')" title="刷新">
        ↻
      </button>
      <button class="icon-btn" @click="toggleDark" title="切换主题">
        {{ isDark ? '☀️' : '🌙' }}
      </button>
    </div>
  </header>
</template>

<style scoped>
.app-header {
  height: 72px;
  padding: 0 32px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-color);
  flex-shrink: 0;
}

.header-left {
  flex: 1;
  min-width: 0;
}

.header-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.2;
  margin: 0;
}

.header-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.icon-btn {
  width: 40px;
  height: 40px;
  border: 1px solid var(--border-color);
  background: var(--bg-card);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-size: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--transition-fast);
}

.icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
  border-color: var(--border-color-hover);
}
</style>
