import { createApp } from 'vue'

import '../style.css'

export const stageLinks = [
  { key: 'signin', label: '登录', href: '/signin.html' },
  { key: 'session', label: '我的会话', href: '/session.html' },
  { key: 'nodes', label: 'BGP 网络节点', href: '/nodes.html' },
  { key: 'peers', label: '互联队列', href: '/peers.html' },
  { key: 'compose', label: '节点互联', href: '/compose.html' },
]

export function mountPage(component: object) {
  createApp(component).mount('#app')
}

export function unauthorized(message: string) {
  const lower = message.toLowerCase()
  return message.includes('401') || lower.includes('unauthorized')
}