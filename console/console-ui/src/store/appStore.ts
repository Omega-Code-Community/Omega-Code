import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAppStore = defineStore('app', () => {
  const locale = ref<string>('zh-CN')

  function setLocale(lang: 'zh-CN' | 'en-US') {
    locale.value = lang
  }

  function resetLocale() {
    locale.value = 'zh-CN'
  }

  return {
    locale,
    setLocale,
    resetLocale
  }
}, {
  persist: true
})