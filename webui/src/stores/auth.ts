// DAPRS WebUI - 认证状态管理
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { getMe, logout, type UserInfo } from '../api'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<UserInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  const initialized = ref(false)

  const isLoggedIn = computed(() => user.value !== null)
  const asn = computed(() => {
    if (!user.value) return null
    const sub = user.value.userinfo?.sub
    return sub ? parseInt(sub as string) : null
  })

  async function fetchUser() {
    // 防止重复请求
    if (loading.value) return

    loading.value = true
    error.value = null
    try {
      const response = await getMe()
      user.value = response.data
    } catch (e) {
      user.value = null
      // 不要打印 401 错误，这是正常的未登录状态
      console.debug('User not authenticated')
    } finally {
      loading.value = false
      initialized.value = true
    }
  }

  async function logoutUser() {
    try {
      await logout()
      user.value = null
      // 刷新页面回到登录状态
      window.location.href = '/'
    } catch (e) {
      console.error('Logout failed:', e)
    }
  }

  return {
    user,
    loading,
    error,
    initialized,
    isLoggedIn,
    asn,
    fetchUser,
    logoutUser
  }
})
