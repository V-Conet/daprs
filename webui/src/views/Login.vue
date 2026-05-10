<script setup lang="ts">
import { useAuthStore } from '../stores/auth'

const authStore = useAuthStore()

function handleLogin() {
  window.location.href = '/api/login'
}
</script>

<template>
  <div class="text-center py-5">
    <h2 class="mb-4">Welcome to DAPRS</h2>
    <p class="text-muted mb-4">DN42 AutoPeering System</p>

    <div v-if="authStore.loading" class="py-3">
      <div class="spinner-border text-primary" role="status">
        <span class="visually-hidden">Loading...</span>
      </div>
    </div>

    <button v-else @click="handleLogin" class="btn btn-primary btn-lg">
      Login with DN42 Auth
    </button>
  </div>
</template>
