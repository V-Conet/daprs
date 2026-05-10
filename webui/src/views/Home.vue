<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useAuthStore } from '../stores/auth'
import { useRouter } from 'vue-router'
import dn42Logo from '../assets/dn42.svg'

const authStore = useAuthStore()
const router = useRouter()

// 主题切换
const isDark = ref(false)

onMounted(async () => {
  // 初始化主题
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
  document.documentElement.setAttribute('data-bs-theme', val ? 'dark' : 'light')
  localStorage.setItem('theme', val ? 'dark' : 'light')
})

function toggleTheme() {
  isDark.value = !isDark.value
}

function handleStart() {
  if (authStore.isLoggedIn) {
    router.push('/dashboard')
  } else {
    // 保存目标，登录后跳转到 dashboard
    sessionStorage.setItem('redirect_after_login', '/dashboard')
    window.location.href = '/api/login'
  }
}
</script>

<template>
  <div class="home">
    <!-- Header -->
    <header class="home-header">
      <div class="container">
        <div class="d-flex justify-content-between align-items-center py-3">
          <span class="brand">VCNET DN42</span>
          <div class="header-links">
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
        <div class="row align-items-center">
          <div class="col-lg-7">
            <span class="asn-badge">AS4242423322</span>
            <h1 class="hero-title">VCNET DN42</h1>
            <p class="hero-subtitle">
              A DN42 Automatic Peering System written in Rust.
            </p>
            <div class="btn-group">
              <button @click="handleStart" class="btn btn-primary btn-lg">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M11.251.068a.5.5 0 0 1 .227.58L9.677 6.5H13a.5.5 0 0 1 .364.843l-8 8.5a.5.5 0 0 1-.842-.49L6.323 9.5H3a.5.5 0 0 1-.364-.843l8-8.5a.5.5 0 0 1 .615-.09z"/>
                </svg>
                Automatic Peering
              </button>
            </div>
          </div>
          <div class="col-lg-5 text-center">
            <img :src="dn42Logo" alt="DN42" class="dn42-logo" />
          </div>
        </div>
      </div>
    </section>

    <!-- Network Info Section -->
    <section class="info-section">
      <div class="container">
        <div class="row g-4">
          <div class="col-md-4">
            <div class="info-card">
              <h5>Network</h5>
              <div class="info-item"><span class="label">ASN</span><span class="value">4242423322</span></div>
              <div class="info-item"><span class="label">IPv4</span><span class="value font-monospace">172.23.100.160/27</span></div>
              <div class="info-item"><span class="label">IPv6</span><span class="value font-monospace">fd48:8669:9f9f::/48</span></div>
            </div>
          </div>
          <div class="col-md-4">
            <div class="info-card">
              <h5>Services</h5>
              <div class="info-item"><span class="label">DNS</span><span class="value font-monospace">anycast.vc.dn42</span></div>
              <div class="info-item"><span class="label">Speed Test</span><span class="value font-monospace">speed.vc.dn42</span></div>
              <div class="info-item"><span class="label">SIP</span><span class="value font-monospace">sip.vc.dn42</span></div>
              <div class="info-item"><span class="label">Domain</span><span class="value font-monospace">vc.dn42</span></div>
            </div>
          </div>
          <div class="col-md-4">
            <div class="info-card">
              <h5>Links</h5>
              <div class="info-item"><span class="label">Looking Glass</span><span class="value"><a href="https://lg-dn42.vconet.top/" target="_blank">lg-dn42.vconet.top</a></span></div>
              <div class="info-item"><span class="label">FlapAlerted</span><span class="value"><a href="https://flap-dn42.vconet.top/" target="_blank">flap-dn42.vconet.top</a></span></div>
              <div class="info-item"><span class="label">Telegram</span><span class="value"><a href="https://t.me/as423322" target="_blank">@as423322</a></span></div>
              <div class="info-item"><span class="label">Email</span><span class="value font-monospace">dn42@vconet.top</span></div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Nodes Section -->
    <section class="nodes-section">
      <div class="container">
        <h3 class="section-title">Global Nodes</h3>
        <div class="row g-3">
          <div class="col-6 col-md-3">
            <div class="node-card">
              <div class="node-name">AMS</div>
              <div class="node-location">Netherlands</div>
              <div class="node-speed">10 Gbps</div>
            </div>
          </div>
          <div class="col-6 col-md-3">
            <div class="node-card">
              <div class="node-name">LAX</div>
              <div class="node-location">United States</div>
              <div class="node-speed">1 Gbps</div>
            </div>
          </div>
          <div class="col-6 col-md-3">
            <div class="node-card">
              <div class="node-name">HKG</div>
              <div class="node-location">Hong Kong</div>
              <div class="node-speed">100 Mbps</div>
            </div>
          </div>
          <div class="col-6 col-md-3">
            <div class="node-card">
              <div class="node-name">DEU</div>
              <div class="node-location">Germany</div>
              <div class="node-speed">1 Gbps</div>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.home {
  min-height: 100vh;
}

/* Header */
.home-header {
  background: var(--bs-body-bg);
  border-bottom: 1px solid var(--bs-border-color);
}

.brand {
  font-weight: 700;
  font-size: 1.1rem;
  color: var(--bs-body-color);
}

.header-links {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.header-link {
  color: var(--bs-secondary-color);
  text-decoration: none;
  font-size: 0.9rem;
  transition: color 0.2s;
}

.header-link:hover {
  color: var(--bs-primary);
}

.theme-btn {
  background: none;
  border: none;
  color: var(--bs-secondary-color);
  cursor: pointer;
  padding: 0.25rem;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition: opacity 0.2s;
}

.theme-btn:hover {
  opacity: 1;
}

/* Hero Section */
.hero {
  padding: 4rem 0;
  background: var(--bs-body-bg);
}

.asn-badge {
  display: inline-block;
  padding: 0.25rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 600;
  background: var(--bs-primary);
  color: #fff;
  border-radius: 1rem;
  margin-bottom: 1rem;
}

.hero-title {
  font-size: 2.5rem;
  font-weight: 700;
  margin-bottom: 1rem;
  color: var(--bs-body-color);
}

.hero-subtitle {
  font-size: 1.1rem;
  color: var(--bs-secondary-color);
  margin-bottom: 1.5rem;
}

.btn-group .btn {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
}

.dn42-logo {
  max-width: 200px;
  opacity: 0.8;
}

/* Info Section */
.info-section {
  padding: 3rem 0;
  background: var(--bs-secondary-bg);
}

.info-card {
  background: var(--bs-body-bg);
  border-radius: 0.5rem;
  padding: 1.25rem;
  height: 100%;
}

.info-card h5 {
  font-size: 0.85rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--bs-primary);
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--bs-border-color);
}

.info-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.35rem 0;
  font-size: 0.85rem;
}

.info-item .label {
  color: var(--bs-secondary-color);
}

.info-item .value {
  font-weight: 500;
}

.info-item a {
  color: var(--bs-primary);
  text-decoration: none;
}

.info-item a:hover {
  text-decoration: underline;
}

/* Nodes Section */
.nodes-section {
  padding: 3rem 0;
}

.section-title {
  font-size: 1.25rem;
  font-weight: 600;
  margin-bottom: 1.5rem;
  color: var(--bs-body-color);
}

.node-card {
  background: var(--bs-secondary-bg);
  border-radius: 0.5rem;
  padding: 1rem;
  text-align: center;
}

.node-name {
  font-size: 1.25rem;
  font-weight: 700;
  color: var(--bs-primary);
}

.node-location {
  font-size: 0.8rem;
  color: var(--bs-secondary-color);
  margin-bottom: 0.25rem;
}

.node-speed {
  font-size: 0.75rem;
  color: var(--bs-success);
  font-weight: 500;
}

@media (max-width: 768px) {
  .hero-title {
    font-size: 1.75rem;
  }

  .dn42-logo {
    max-width: 120px;
    margin-bottom: 2rem;
  }
}
</style>
