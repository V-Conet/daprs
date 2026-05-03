import { computed, createApp, ref } from 'vue'

import '../style.css'

export const stageLinks = [
  { key: 'signin', label: '登录', href: '/signin.html' },
  { key: 'session', label: '我的会话', href: '/session.html' },
  { key: 'nodes', label: 'BGP 网络节点', href: '/nodes.html' },
  { key: 'peers', label: '互联队列', href: '/peers.html' },
  { key: 'compose', label: '节点互联', href: '/compose.html' },
]

const THEME_KEY = 'daprs-theme'

export type ThemeMode = 'light' | 'dark' | 'system'

export function applyTheme(theme: ThemeMode) {
  const root = document.documentElement
  if (theme === 'system') {
    root.removeAttribute('data-theme')
    return
  }

  root.setAttribute('data-theme', theme)
}

export function getStoredTheme(): ThemeMode {
  const theme = localStorage.getItem(THEME_KEY)
  if (theme === 'light' || theme === 'dark' || theme === 'system') {
    return theme
  }
  return 'system'
}

export function mountPage(component: object) {
  applyTheme(getStoredTheme())
  createApp(component).mount('#app')
}

export function unauthorized(message: string) {
  const lower = message.toLowerCase()
  return message.includes('401') || lower.includes('unauthorized')
}

export function useThemeToggle() {
  const theme = ref<ThemeMode>(getStoredTheme())
  applyTheme(theme.value)

  const themeLabel = computed(() => {
    if (theme.value === 'dark') {
      return '深色'
    }
    if (theme.value === 'light') {
      return '浅色'
    }
    return '跟随系统'
  })

  function toggleTheme() {
    if (theme.value === 'system') {
      theme.value = 'light'
    } else if (theme.value === 'light') {
      theme.value = 'dark'
    } else {
      theme.value = 'system'
    }

    localStorage.setItem(THEME_KEY, theme.value)
    applyTheme(theme.value)
  }

  return {
    themeLabel,
    toggleTheme,
  }
}