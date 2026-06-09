<script setup lang="ts">
import { useAuthStore } from '../stores/auth'
import { ref, onMounted, watch } from 'vue'
import { checkAdmin } from '../api'

const authStore = useAuthStore()
const isDark = ref(false)
const isAdmin = ref(false)

onMounted(async () => {
  // 初始化主题
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme === 'dark') {
    isDark.value = true
    document.documentElement.setAttribute('data-theme', 'dark')
  } else if (savedTheme === 'light') {
    isDark.value = false
    document.documentElement.setAttribute('data-theme', 'light')
  } else {
    isDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
    document.documentElement.setAttribute('data-theme', isDark.value ? 'dark' : 'light')
  }

  // 检查管理员状态
  if (authStore.isLoggedIn) {
    try {
      const response = await checkAdmin()
      isAdmin.value = response.data
    } catch {
      isAdmin.value = false
    }
  }
})

watch(isDark, (val) => {
  document.documentElement.setAttribute('data-theme', val ? 'dark' : 'light')
  localStorage.setItem('theme', val ? 'dark' : 'light')
})

function toggleTheme() {
  isDark.value = !isDark.value
}

function handleLogin() {
  window.location.href = '/api/login'
}
</script>

<template>
  <nav class="navbar">
    <div class="navbar-inner">
      <router-link to="/" class="brand">
        DAPRS
      </router-link>

      <div class="navbar-links">
        <a href="https://lg-dn42.vconet.top/" target="_blank" class="nav-link-external">
          Looking Glass
        </a>
        <a href="https://flap-dn42.vconet.top/" target="_blank" class="nav-link-external">
          FlapAlerted
        </a>

        <button @click="toggleTheme" class="theme-toggle" :title="isDark ? 'Switch to light mode' : 'Switch to dark mode'">
          <svg v-if="isDark" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="5"></circle>
            <line x1="12" y1="1" x2="12" y2="3"></line>
            <line x1="12" y1="21" x2="12" y2="23"></line>
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
            <line x1="1" y1="12" x2="3" y2="12"></line>
            <line x1="21" y1="12" x2="23" y2="12"></line>
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
          </svg>
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
          </svg>
        </button>

        <template v-if="authStore.isLoggedIn">
          <span class="user-asn">AS{{ authStore.asn }}</span>
          <router-link v-if="isAdmin" to="/admin" class="nav-link-icon" title="Admin">
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
            </svg>
          </router-link>
          <router-link to="/tools" class="nav-link">Tools</router-link>
          <button @click="authStore.logoutUser" class="btn-logout">
            Logout
          </button>
        </template>
        <template v-else>
          <button @click="handleLogin" class="btn-login">
            Login
          </button>
        </template>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.navbar {
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-color);
  padding: 0 var(--space-lg);
  position: sticky;
  top: 0;
  z-index: 100;
}

.navbar-inner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  height: 56px;
  max-width: 1200px;
  margin: 0 auto;
}

.brand {
  font-size: 1.125rem;
  font-weight: 700;
  color: var(--text-primary);
  text-decoration: none;
  letter-spacing: 0.02em;
}

.brand:hover {
  color: var(--accent);
}

.navbar-links {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.nav-link-external {
  color: var(--text-tertiary);
  font-size: 0.875rem;
  text-decoration: none;
  transition: color var(--transition-fast);
}

.nav-link-external:hover {
  color: var(--text-secondary);
}

.nav-link {
  color: var(--text-secondary);
  font-size: 0.875rem;
  text-decoration: none;
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.nav-link:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.nav-link-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-xs);
  color: var(--text-secondary);
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.nav-link-icon:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.theme-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--space-xs);
  background: none;
  border: none;
  color: var(--text-tertiary);
  cursor: pointer;
  border-radius: var(--radius-sm);
  transition: all var(--transition-fast);
}

.theme-toggle:hover {
  color: var(--text-secondary);
  background: var(--bg-hover);
}

.user-asn {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-tertiary);
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-tertiary);
  border-radius: var(--radius-full);
}

.btn-login {
  padding: var(--space-sm) var(--space-md);
  background: var(--accent);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-login:hover {
  background: var(--accent-hover);
}

.btn-logout {
  padding: var(--space-sm) var(--space-md);
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-logout:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

@media (max-width: 768px) {
  .navbar {
    padding: 0 var(--space-md);
  }

  .nav-link-external {
    display: none;
  }

  .navbar-links {
    gap: var(--space-sm);
  }
}
</style>
