<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { authApi } from '@/api/auth'
import { useAuthStore } from '@/stores/auth'

const router = useRouter()
const message = useMessage()
const authStore = useAuthStore()

const isLogin = ref(true)
const username = ref('')
const password = ref('')
const loading = ref(false)

async function handleSubmit() {
  if (!username.value || !password.value) {
    message.warning('请填写用户名和密码')
    return
  }

  loading.value = true
  try {
    if (isLogin.value) {
      const res = await authApi.login({
        username: username.value,
        password: password.value,
      })
      authStore.setAuth(res.data.token, res.data.user)
      message.success('登录成功')
      router.push('/')
    } else {
      await authApi.register({
        username: username.value,
        password: password.value,
      })
      message.success('注册成功，请登录')
      isLogin.value = true
    }
  } catch (e: unknown) {
    const err = e as Error
    message.error(err.message || '操作失败')
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <div class="login-header">
        <n-gradient-text type="info" :size="28" class="logo">
          NetPulse Web
        </n-gradient-text>
        <p class="subtitle">网络质量测试平台</p>
      </div>

      <n-form class="login-form">
        <n-form-item label="用户名">
          <n-input
            v-model:value="username"
            placeholder="请输入用户名"
            :disabled="loading"
            size="large"
          />
        </n-form-item>
        <n-form-item label="密码">
          <n-input
            v-model:value="password"
            type="password"
            placeholder="请输入密码"
            :disabled="loading"
            size="large"
            @keyup.enter="handleSubmit"
          />
        </n-form-item>
        <n-button
          type="primary"
          block
          size="large"
          :loading="loading"
          @click="handleSubmit"
        >
          {{ isLogin ? '登录' : '注册' }}
        </n-button>
      </n-form>

      <div class="login-footer">
        <n-button
          text
          type="primary"
          @click="isLogin = !isLogin"
        >
          {{ isLogin ? '没有账号？去注册' : '已有账号？去登录' }}
        </n-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-color-body);
}
.login-card {
  width: 420px;
  padding: 40px;
  border-radius: 12px;
  background: var(--n-color-card);
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
}
.login-header {
  text-align: center;
  margin-bottom: 32px;
}
.logo {
  font-weight: 700;
}
.subtitle {
  margin-top: 8px;
  color: var(--n-text-color-3);
  font-size: 14px;
}
.login-form {
  margin-bottom: 16px;
}
.login-footer {
  text-align: center;
}
</style>
