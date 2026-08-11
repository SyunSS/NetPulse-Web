import { createApp, defineComponent, h } from 'vue'
import { createPinia } from 'pinia'
import { darkTheme, dateZhCN, NConfigProvider, NDialogProvider, NMessageProvider, zhCN } from 'naive-ui'
import App from './App.vue'
import router from './router'
import './assets/styles/main.css'
import { useDark } from './utils/theme'

const lightOverrides = {
  common: {
    primaryColor: '#138f88',
    primaryColorHover: '#0f766f',
    primaryColorPressed: '#0b5d58',
    primaryColorSuppl: '#61cfc4',
    borderRadius: '8px',
    fontFamily: "'DM Sans', 'PingFang SC', 'Microsoft YaHei', sans-serif",
    fontSize: '14px',
    textColor1: '#102033',
    textColor2: '#4e687d',
    textColor3: '#7890a2',
    borderColor: 'rgba(27,70,98,0.17)',
    dividerColor: 'rgba(27,70,98,0.14)',
    bodyColor: '#eef4f8',
    cardColor: '#f9fcfd',
    hoverColor: 'rgba(27,92,120,0.07)',
    inputColor: '#f8fbfc',
    placeholderColor: '#7890a2',
    tableColor: '#f9fcfd',
    tableHeaderColor: '#e6eef4',
    actionColor: '#e6eef4',
    successColor: '#138f88',
    warningColor: '#c97916',
    errorColor: '#d75d41',
  },
  Card: { borderRadius: '14px', paddingMedium: '24px', color: '#f9fcfd', borderColor: 'rgba(27,70,98,0.17)' },
  Button: { borderRadius: '6px', fontWeight: '600', heightMedium: '36px', paddingMedium: '8px 16px' },
  Input: { borderRadius: '6px', border: '1px solid rgba(27,70,98,0.17)', borderFocus: '1px solid #138f88', color: '#f8fbfc', heightMedium: '36px' },
  Select: { borderRadius: '6px', border: '1px solid rgba(27,70,98,0.17)', borderFocus: '1px solid #138f88', color: '#f8fbfc', menuColor: '#f9fcfd', heightMedium: '36px' },
  Tag: { borderRadius: '9999px', fontSize: '11px', fontWeight: '600', heightMedium: '24px' },
  Dialog: { borderRadius: '14px', titleFontSize: '18px', titleFontWeight: '700' },
  Message: { borderRadius: '8px' },
}

const darkOverrides = {
  common: {
    primaryColor: '#61e7d2',
    primaryColorHover: '#9af5e5',
    primaryColorPressed: '#3dbfaf',
    primaryColorSuppl: '#61e7d2',
    borderRadius: '8px',
    fontFamily: "'DM Sans', 'PingFang SC', 'Microsoft YaHei', sans-serif",
    fontSize: '14px',
    textColor1: '#edf6ff',
    textColor2: '#9bb0c2',
    textColor3: '#61788e',
    borderColor: 'rgba(136,169,201,0.2)',
    dividerColor: 'rgba(136,169,201,0.16)',
    bodyColor: '#08111f',
    cardColor: '#0f2032',
    hoverColor: 'rgba(97,231,210,0.08)',
    inputColor: '#0a1828',
    placeholderColor: '#61788e',
    tableColor: '#0f2032',
    tableHeaderColor: '#12263a',
    actionColor: '#12263a',
    successColor: '#61e7d2',
    warningColor: '#ffb454',
    errorColor: '#ff8a65',
  },
  Card: { borderRadius: '14px', paddingMedium: '24px', color: '#0f2032', borderColor: 'rgba(136,169,201,0.2)' },
  Button: { borderRadius: '6px', fontWeight: '600', heightMedium: '36px', paddingMedium: '8px 16px' },
  Input: { borderRadius: '6px', border: '1px solid rgba(136,169,201,0.2)', borderFocus: '1px solid #61e7d2', color: '#0a1828', heightMedium: '36px' },
  Select: { borderRadius: '6px', border: '1px solid rgba(136,169,201,0.2)', borderFocus: '1px solid #61e7d2', color: '#0a1828', menuColor: '#12263a', heightMedium: '36px' },
  Tag: { borderRadius: '9999px', fontSize: '11px', fontWeight: '600', heightMedium: '24px' },
  Dialog: { borderRadius: '14px', titleFontSize: '18px', titleFontWeight: '700' },
  Message: { borderRadius: '8px' },
}

const RootProviders = defineComponent({
  setup() {
    const { isDark } = useDark()
    return () => h(NConfigProvider, {
      theme: isDark.value ? darkTheme : null,
      themeOverrides: isDark.value ? darkOverrides : lightOverrides,
      locale: zhCN,
      dateLocale: dateZhCN,
    }, {
      default: () => h(NMessageProvider, null, {
        default: () => h(NDialogProvider, null, { default: () => h(App) }),
      }),
    })
  },
})

const app = createApp(RootProviders)
app.use(createPinia())
app.use(router)
app.mount('#app')
