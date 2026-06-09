<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useRouter } from 'vue-router'

const authStore = useAuthStore()
const router = useRouter()
const isDark = ref(false)

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

  // 获取用户信息
  if (!authStore.user && !authStore.loading) {
    await authStore.fetchUser()
  }

  // 检查是否有登录后的跳转目标
  const redirect = sessionStorage.getItem('redirect_after_login')
  if (redirect && authStore.isLoggedIn) {
    sessionStorage.removeItem('redirect_after_login')
    router.push(redirect)
  }
})

watch(isDark, (val) => {
  document.documentElement.setAttribute('data-theme', val ? 'dark' : 'light')
  localStorage.setItem('theme', val ? 'dark' : 'light')
})

function toggleTheme() {
  isDark.value = !isDark.value
}

function handleStart() {
  if (authStore.isLoggedIn) {
    router.push('/dashboard')
  } else {
    sessionStorage.setItem('redirect_after_login', '/dashboard')
    window.location.href = '/api/login'
  }
}

const nodes = [
  { name: 'AMS', location: 'Netherlands', speed: '10 Gbps' },
  { name: 'LAX', location: 'United States', speed: '1 Gbps' },
  { name: 'HKG', location: 'Hong Kong', speed: '100 Mbps' },
  { name: 'DEU', location: 'Germany', speed: '1 Gbps' },
]
</script>

<template>
  <div class="home">
    <!-- Header -->
    <header class="home-header">
      <div class="container">
        <div class="header-inner">
          <span class="brand">VCNET DN42</span>
          <div class="header-actions">
            <a href="https://blog.vconet.top/dn42/" target="_blank" class="header-link">DN42 Nodes</a>
            <button @click="toggleTheme" class="theme-btn" :title="isDark ? 'Switch to light mode' : 'Switch to dark mode'">
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
          </div>
        </div>
      </div>
    </header>

    <!-- Hero Section -->
    <section class="hero">
      <div class="container">
        <div class="hero-content">
          <div class="hero-badge">AS4242423322</div>
          <h1 class="hero-title">VCNET DN42</h1>
          <p class="hero-subtitle">
            A DN42 Automatic Peering System written in Rust
          </p>
          <div class="hero-actions">
            <button @click="handleStart" class="btn-primary-large">
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
                <path d="M11.251.068a.5.5 0 0 1 .227.58L9.677 6.5H13a.5.5 0 0 1 .364.843l-8 8.5a.5.5 0 0 1-.842-.49L6.323 9.5H3a.5.5 0 0 1-.364-.843l8-8.5a.5.5 0 0 1 .615-.09z"/>
              </svg>
              Automatic Peering
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- Network Info -->
    <section class="info-section">
      <div class="container">
        <div class="info-grid">
          <div class="info-block">
            <h3 class="info-title">Network</h3>
            <div class="info-list">
              <div class="info-item">
                <span class="info-label">ASN</span>
                <span class="info-value">4242423322</span>
              </div>
              <div class="info-item">
                <span class="info-label">IPv4</span>
                <span class="info-value mono">172.23.100.160/27</span>
              </div>
              <div class="info-item">
                <span class="info-label">IPv6</span>
                <span class="info-value mono">fd48:8669:9f9f::/48</span>
              </div>
            </div>
          </div>

          <div class="info-block">
            <h3 class="info-title">Services</h3>
            <div class="info-list">
              <div class="info-item">
                <span class="info-label">DNS</span>
                <span class="info-value mono">anycast.vc.dn42</span>
              </div>
              <div class="info-item">
                <span class="info-label">Speed Test</span>
                <span class="info-value mono">speed.vc.dn42</span>
              </div>
              <div class="info-item">
                <span class="info-label">SIP</span>
                <span class="info-value mono">sip.vc.dn42</span>
              </div>
              <div class="info-item">
                <span class="info-label">Domain</span>
                <span class="info-value mono">vc.dn42</span>
              </div>
            </div>
          </div>

          <div class="info-block">
            <h3 class="info-title">Links</h3>
            <div class="info-list">
              <div class="info-item">
                <span class="info-label">Looking Glass</span>
                <a href="https://lg-dn42.vconet.top/" target="_blank" class="info-link">lg-dn42.vconet.top</a>
              </div>
              <div class="info-item">
                <span class="info-label">FlapAlerted</span>
                <a href="https://flap-dn42.vconet.top/" target="_blank" class="info-link">flap-dn42.vconet.top</a>
              </div>
              <div class="info-item">
                <span class="info-label">Telegram</span>
                <a href="https://t.me/as423322" target="_blank" class="info-link">@as423322</a>
              </div>
              <div class="info-item">
                <span class="info-label">Email</span>
                <span class="info-value mono">dn42@vconet.top</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Nodes Section -->
    <section class="nodes-section">
      <div class="container">
        <h3 class="section-title">Global Nodes</h3>
        <div class="nodes-grid">
          <div v-for="node in nodes" :key="node.name" class="node-card">
            <div class="node-name">{{ node.name }}</div>
            <div class="node-location">{{ node.location }}</div>
            <div class="node-speed">{{ node.speed }}</div>
          </div>
        </div>
      </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
      <div class="container">
        <p class="footer-text">
          Powered by Rust · DN42 Network
        </p>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.home {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

/* Header */
.home-header {
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-color);
  padding: var(--space-md) 0;
}

.header-inner {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.brand {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-primary);
}

.header-actions {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}

.header-link {
  font-size: 0.875rem;
  color: var(--text-tertiary);
  text-decoration: none;
  transition: color var(--transition-fast);
}

.header-link:hover {
  color: var(--text-secondary);
}

.theme-btn {
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

.theme-btn:hover {
  color: var(--text-secondary);
  background: var(--bg-hover);
}

/* Hero */
.hero {
  padding: var(--space-3xl) 0;
  background: var(--bg-primary);
}

.hero-content {
  max-width: 600px;
}

.hero-badge {
  display: inline-block;
  padding: var(--space-xs) var(--space-md);
  font-size: 0.75rem;
  font-weight: 600;
  background: var(--accent-light);
  color: var(--accent);
  border-radius: var(--radius-full);
  margin-bottom: var(--space-lg);
}

.hero-title {
  font-size: 3rem;
  font-weight: 700;
  margin-bottom: var(--space-md);
  letter-spacing: -0.02em;
}

.hero-subtitle {
  font-size: 1.125rem;
  color: var(--text-secondary);
  margin-bottom: var(--space-xl);
  line-height: 1.6;
}

.btn-primary-large {
  display: inline-flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-xl);
  background: var(--accent);
  color: var(--text-inverse);
  border: none;
  border-radius: var(--radius-md);
  font-size: 1rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-primary-large:hover {
  background: var(--accent-hover);
  transform: translateY(-1px);
}

/* Info Section */
.info-section {
  padding: var(--space-2xl) 0;
  background: var(--bg-secondary);
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-xl);
}

.info-block {
  padding: var(--space-lg);
  background: var(--bg-primary);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-color);
}

.info-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--accent);
  margin-bottom: var(--space-md);
}

.info-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-xs) 0;
}

.info-label {
  font-size: 0.875rem;
  color: var(--text-tertiary);
}

.info-value {
  font-size: 0.875rem;
  color: var(--text-primary);
  font-weight: 500;
}

.info-value.mono {
  font-family: var(--font-mono);
  font-size: 0.8rem;
}

.info-link {
  font-size: 0.875rem;
  color: var(--accent);
  text-decoration: none;
}

.info-link:hover {
  text-decoration: underline;
}

/* Nodes Section */
.nodes-section {
  padding: var(--space-2xl) 0;
  background: var(--bg-primary);
}

.section-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-lg);
}

.nodes-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-md);
}

.node-card {
  padding: var(--space-lg);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  text-align: center;
}

.node-name {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--accent);
  margin-bottom: var(--space-xs);
}

.node-location {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin-bottom: var(--space-xs);
}

.node-speed {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--success);
}

/* Footer */
.footer {
  padding: var(--space-xl) 0;
  background: var(--bg-primary);
  border-top: 1px solid var(--border-color);
  margin-top: auto;
}

.footer-text {
  font-size: 0.875rem;
  color: var(--text-tertiary);
  text-align: center;
}

/* Responsive */
@media (max-width: 1024px) {
  .info-grid {
    grid-template-columns: 1fr;
  }

  .nodes-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 768px) {
  .hero {
    padding: var(--space-2xl) 0;
  }

  .hero-title {
    font-size: 2rem;
  }

  .hero-subtitle {
    font-size: 1rem;
  }

  .nodes-grid {
    grid-template-columns: repeat(2, 1fr);
  }

  .header-link {
    display: none;
  }
}
</style>
