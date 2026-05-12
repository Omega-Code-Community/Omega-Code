import { createApp } from 'vue'
import './style.css'
import App from './App.vue'
import { createPinia } from 'pinia'
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate'
import { createI18n } from 'vue-i18n'
import zhCN from './i18n/zh-cn'
import enUS from './i18n/en-us'

const pinia = createPinia()
pinia.use(piniaPluginPersistedstate)

const localStore = localStorage.getItem('app')
const savedLocale = localStore ? JSON.parse(localStore).locale : 'zh-CN'

const i18n = createI18n({
    legacy: false,
    locale: savedLocale,
    messages: {
        'zh-CN': zhCN,
        'en-US': enUS,
    },
})

const app = createApp(App)
app.use(i18n)
app.use(pinia)
app.mount('#app')
