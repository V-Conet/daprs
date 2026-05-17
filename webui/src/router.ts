// DAPRS WebUI - 路由配置
import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from './stores/auth'

const routes = [
  {
    path: '/',
    name: 'Home',
    component: () => import('./views/Home.vue'),
    meta: { requiresAuth: false }
  },
  {
    path: '/dashboard',
    name: 'Dashboard',
    component: () => import('./views/Dashboard.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/peering/:node',
    name: 'Peering',
    component: () => import('./views/Peering.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/peer/:node',
    name: 'PeerInfo',
    component: () => import('./views/PeerInfo.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/tools',
    name: 'Tools',
    component: () => import('./views/Tools.vue'),
    meta: { requiresAuth: true }
  },
  {
    path: '/admin',
    name: 'Admin',
    component: () => import('./views/Admin.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/admin/modify/:node/:asn',
    name: 'AdminModify',
    component: () => import('./views/AdminModify.vue'),
    meta: { requiresAuth: true, requiresAdmin: true }
  },
  {
    path: '/login',
    name: 'Login',
    component: () => import('./views/Login.vue'),
    meta: { requiresAuth: false }
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

// 路由守卫 - Vue Router 4 风格
// 返回 false 取消导航，返回路由地址进行重定向，不返回则继续导航
router.beforeEach(async (to) => {
  const authStore = useAuthStore()

  // 不需要认证的页面直接放行
  if (!to.meta.requiresAuth) {
    return true
  }

  // 需要认证的页面
  // 如果还没获取用户信息，先获取
  if (!authStore.user && !authStore.loading) {
    await authStore.fetchUser()
  }

  // 未登录则重定向到登录页
  if (!authStore.isLoggedIn) {
    // 直接跳转到 OAuth 登录
    window.location.href = '/api/login'
    return false
  }

  // 已登录，放行
  return true
})

export default router
