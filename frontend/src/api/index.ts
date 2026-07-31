import axios from 'axios'
import { useAuthStore } from '@/stores/auth'
import router from '@/router'

export function getErrorMessage(error: unknown, fallback = '请求失败'): string {
  const response = (error as { response?: { data?: unknown } })?.response
  const data = response?.data
  if (data && typeof data === 'object' && 'msg' in data && typeof data.msg === 'string') {
    return data.msg
  }
  if (error instanceof Error && error.message) return error.message
  return fallback
}

const http = axios.create({
  baseURL: '/api',
  timeout: 30000,
})

// 请求拦截器：自动附加 Token
http.interceptors.request.use(
  (config) => {
    const authStore = useAuthStore()
    if (authStore.token) {
      config.headers.Authorization = `Bearer ${authStore.token}`
    }
    return config
  },
  (error) => Promise.reject(error),
)

// 响应拦截器：统一错误处理
http.interceptors.response.use(
  (response) => {
    const { data } = response
    // blob 响应直接返回，不经过 JSON 检查
    if (data instanceof Blob) {
      return data
    }
    if (data.code !== 0) {
      return Promise.reject(new Error(data.msg || '请求失败'))
    }
    return data
  },
  (error) => {
    if (error.response?.status === 401) {
      const authStore = useAuthStore()
      authStore.logout()
      router.push('/login')
    }
    const data = error.response?.data
    if (data instanceof Blob) {
      return data.text().then((text) => {
        try {
          const parsed = JSON.parse(text)
          return Promise.reject(new Error(parsed.msg || '请求失败'))
        } catch {
          return Promise.reject(new Error(error.message || '请求失败'))
        }
      })
    }
    return Promise.reject(new Error(data?.msg || error.message || '请求失败'))
  },
)

export default http
