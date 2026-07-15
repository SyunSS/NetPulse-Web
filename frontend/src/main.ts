import { createApp, defineComponent, h } from 'vue'
import { createPinia } from 'pinia'
import { darkTheme, zhCN, dateZhCN, NConfigProvider, NMessageProvider, NDialogProvider } from 'naive-ui'
import App from './App.vue'
import router from './router'
import './assets/styles/main.css'
import { useDark } from './utils/theme'

// 创建一个根级 Provider 包装组件，确保所有页面都能访问
const RootProviders = defineComponent({
  setup() {
    const { isDark } = useDark()
    return () =>
      h(NConfigProvider, { theme: isDark.value ? darkTheme : null, locale: zhCN, dateLocale: dateZhCN }, {
        default: () =>
          h(NMessageProvider, null, {
            default: () =>
              h(NDialogProvider, null, {
                default: () => h(App),
              }),
          }),
      })
  },
})

const app = createApp(RootProviders)
const pinia = createPinia()
app.use(pinia)
app.use(router)
app.mount('#app')
