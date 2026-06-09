<script setup lang="ts">
import { useAuthStore } from '../stores/auth'

const authStore = useAuthStore()

function handleLogin() {
  window.location.href = '/api/login'
}
</script>

<template>
  <div class="login-page">
    <div class="login-card">
      <h2>Welcome to DAPRS</h2>
      <p class="subtitle">DN42 AutoPeering System</p>

      <div v-if="authStore.loading" class="loading-state">
        <div class="spinner"></div>
      </div>

      <button v-else @click="handleLogin" class="btn-login">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"></path>
          <polyline points="10 17 15 12 10 7"></polyline>
          <line x1="15" y1="12" x2="3" y2="12"></line>
        </svg>
        Login with DN42 Auth
      </button>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: calc(100vh - 120px);
  padding: var(--space-xl);
}

.login-card {
  text-align: center;
  padding: var(--space-2xl);
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-lg);
  max-width: 400px;
  width: 100%;
}

.login-card h2 {
  font-size: 1.5rem;
  font-weight: 600;
  margin-bottom: var(--space-sm);
}

.subtitle {
  color: var(--text-secondary);
  margin-bottom: var(--space-xl);
}

.loading-state {
  padding: var(--space-lg);
}

.spinner {
  width: 32px;
  height: 32px;
  margin: 0 auto;
  border: 3px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.btn-login {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  width: 100%;
  padding: var(--space-md) var(--space-xl);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-md);
  color: var(--text-inverse);
  font-size: 1rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-login:hover {
  background: var(--accent-hover);
  transform: translateY(-1px);
}
</style>
