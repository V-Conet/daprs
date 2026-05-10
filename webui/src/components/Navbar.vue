<script setup lang="ts">
import { useAuthStore } from '../stores/auth'
import { ref, onMounted, watch } from 'vue'

const authStore = useAuthStore()

// 主题切换
const isDark = ref(false)

onMounted(() => {
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme === 'dark') {
    isDark.value = true
    document.documentElement.setAttribute('data-bs-theme', 'dark')
  } else if (savedTheme === 'light') {
    isDark.value = false
    document.documentElement.setAttribute('data-bs-theme', 'light')
  } else {
    isDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
    document.documentElement.setAttribute('data-bs-theme', isDark.value ? 'dark' : 'light')
  }
})

watch(isDark, (val) => {
  document.documentElement.setAttribute('data-bs-theme', val ? 'dark' : 'light')
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
  <nav class="navbar navbar-expand-lg sticky-top" :class="isDark ? 'navbar-dark bg-dark' : 'navbar-light bg-white border-bottom'">
    <div class="container">
      <router-link to="/" class="navbar-brand fw-bold">
        DAPRS
      </router-link>

      <div class="navbar-nav ms-auto d-flex flex-row align-items-center">
        <!-- 主题切换 -->
        <button @click="toggleTheme" class="btn btn-link nav-link p-0 me-2" :title="isDark ? 'Switch to light mode' : 'Switch to dark mode'">
          <svg v-if="isDark" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
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
          <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
          </svg>
        </button>

        <template v-if="authStore.isLoggedIn">
          <span class="navbar-text me-2 small" :class="isDark ? 'text-light' : 'text-muted'">
            AS{{ authStore.asn }}
          </span>
          <router-link to="/tools" class="nav-link">Tools</router-link>
          <button @click="authStore.logoutUser" class="btn btn-outline-primary btn-sm ms-2">
            Logout
          </button>
        </template>
        <template v-else>
          <button @click="handleLogin" class="btn btn-primary btn-sm">
            Login
          </button>
        </template>
      </div>
    </div>
  </nav>
</template>

<style scoped>
.navbar-brand {
  font-weight: 600;
  letter-spacing: 0.05em;
}

.btn-link {
  text-decoration: none;
  opacity: 0.7;
}

.btn-link:hover {
  opacity: 1;
}
</style>
