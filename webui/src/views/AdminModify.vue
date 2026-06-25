<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getNodes, adminModifyPeer, type NodeAgentConfig, type PeeringPayload, type PeerInfoResponse } from '../api'

const route = useRoute()
const router = useRouter()

const nodeName = route.params.node as string
const targetAsn = Number(route.params.asn)

const node = ref<NodeAgentConfig | null>(null)
const existingConfig = ref<PeerInfoResponse | null>(null)
const loading = ref(true)
const submitting = ref(false)
const error = ref<string | null>(null)

const form = ref<PeeringPayload>({
  is_mhp: true,
  is_nhp: true,
  v4: null,
  v6: null,
  lla: null,
  is_prefer_lla: false,
  endpoint: '',
  pubkey: '',
  custom_port: null,
  psk: null,
  mtu: 1420
})

function onMhpChange(value: boolean) {
  if (!value) {
    form.value.is_nhp = false
  }
}

const defaultPort = computed(() => {
  if (!targetAsn) return null
  if (targetAsn >= 4242420000) {
    return targetAsn % 100000
  } else {
    return 40000 + (targetAsn % 10000)
  }
})

onMounted(async () => {
  try {
    const response = await getNodes()
    node.value = response.data[nodeName] || null

    if (!node.value) {
      error.value = `Node "${nodeName}" not found`
      loading.value = false
      return
    }

    const peersResp = await fetch(`/api/admin/peers`, { credentials: 'include' })
    if (peersResp.ok) {
      const allPeers = await peersResp.json()
      const peerList = allPeers[nodeName] || []
      const peer = peerList.find((p: any) => p.asn === targetAsn)
      if (peer && peer.wg) {
        existingConfig.value = peer
        form.value = {
          is_mhp: peer.bird?.is_mhp ?? true,
          is_nhp: peer.bird?.is_nhp ?? true,
          v4: peer.wg.peer_v4,
          v6: peer.wg.peer_v6,
          lla: null,
          is_prefer_lla: false,
          endpoint: peer.wg.endpoint || '',
          pubkey: peer.wg.pubkey,
          custom_port: peer.wg.port || null,
          psk: peer.wg.psk,
          mtu: peer.wg.mtu || 1420
        }
      }
    }
  } catch (e) {
    error.value = 'Failed to load peer info'
    console.error(e)
  } finally {
    loading.value = false
  }
})

async function submitForm() {
  submitting.value = true
  error.value = null

  if (form.value.custom_port !== null) {
    if (isNaN(form.value.custom_port) || form.value.custom_port < 1024 || form.value.custom_port > 65535) {
      error.value = 'Port must be between 1024 and 65535'
      submitting.value = false
      return
    }
  }
  if (form.value.mtu !== null) {
    if (isNaN(form.value.mtu) || form.value.mtu < 576 || form.value.mtu > 9000) {
      error.value = 'MTU must be between 576 and 9000'
      submitting.value = false
      return
    }
  }

  const payload: PeeringPayload = {
    is_mhp: form.value.is_mhp,
    is_nhp: form.value.is_nhp,
    v4: form.value.v4 || null,
    v6: form.value.v6 || null,
    lla: form.value.lla || null,
    is_prefer_lla: form.value.is_prefer_lla,
    endpoint: form.value.endpoint,
    pubkey: form.value.pubkey,
    custom_port: form.value.custom_port ? Math.floor(form.value.custom_port) : null,
    psk: form.value.psk || null,
    mtu: form.value.mtu ? Math.floor(form.value.mtu) : null
  }

  try {
    await adminModifyPeer({
      node: nodeName,
      asn: targetAsn,
      payload
    })
    router.push('/admin')
  } catch (e: any) {
    error.value = e.response?.data?.error || 'Failed to modify peer'
  } finally {
    submitting.value = false
  }
}

function goBack() {
  router.back()
}
</script>

<template>
  <div class="modify-page">
    <div class="page-header">
      <button @click="goBack" class="btn-back">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="19" y1="12" x2="5" y2="12"></line>
          <polyline points="12 19 5 12 12 5"></polyline>
        </svg>
      </button>
      <h2>Modify Peer · {{ nodeName }} / AS{{ targetAsn }}</h2>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <div v-else-if="error && !node" class="error-state">
      {{ error }}
    </div>

    <template v-else-if="node">
      <form @submit.prevent="submitForm" class="form-container">
        <div class="info-banner">
          <span>Node: <strong>{{ nodeName }}</strong></span>
          <span>Target ASN: <strong>AS{{ targetAsn }}</strong></span>
        </div>

        <!-- WireGuard -->
        <div class="form-section">
          <h3 class="section-title">WireGuard Configuration</h3>
          <div class="form-grid">
            <div class="form-group">
              <label class="form-label">Endpoint *</label>
              <input v-model="form.endpoint" type="text" class="form-input mono" placeholder="example.com:51820" required />
            </div>
            <div class="form-group">
              <label class="form-label">Public Key *</label>
              <input v-model="form.pubkey" type="text" class="form-input mono" placeholder="WireGuard public key" required />
            </div>
            <div class="form-group">
              <label class="form-label">Pre-Shared Key</label>
              <input v-model="form.psk" type="text" class="form-input mono" placeholder="Optional PSK" />
            </div>
            <div class="form-group">
              <label class="form-label">MTU</label>
              <input v-model.number="form.mtu" type="number" class="form-input" min="576" max="9000" />
            </div>
            <div class="form-group">
              <label class="form-label">Port</label>
              <input v-model.number="form.custom_port" type="number" class="form-input" min="1024" max="65535" :placeholder="`Default: ${defaultPort}`" />
            </div>
          </div>
        </div>

        <!-- IP -->
        <div class="form-section">
          <h3 class="section-title">IP Address Configuration</h3>
          <div class="form-grid">
            <div class="form-group">
              <label class="form-label">Tunnel IPv4</label>
              <input v-model="form.v4" type="text" class="form-input mono" placeholder="DN42 IPv4 address" />
            </div>
            <div class="form-group">
              <label class="form-label">Tunnel IPv6</label>
              <input v-model="form.v6" type="text" class="form-input mono" placeholder="DN42 IPv6 or Link-Local" />
            </div>
          </div>
        </div>

        <!-- BGP -->
        <div class="form-section">
          <h3 class="section-title">BGP Extensions</h3>
          <div class="toggle-group">
            <label class="toggle-item">
              <input type="checkbox" v-model="form.is_mhp" @change="onMhpChange(form.is_mhp)" />
              <div class="toggle-content">
                <span class="toggle-title">Multiprotocol BGP (MP-BGP)</span>
                <span class="toggle-desc">Single BGP session for both IPv4 and IPv6 routes</span>
              </div>
            </label>
            <label class="toggle-item">
              <input type="checkbox" v-model="form.is_nhp" :disabled="!form.is_mhp" />
              <div class="toggle-content">
                <span class="toggle-title">Extended Next Hop (IPv6 session)</span>
                <span class="toggle-desc">Use IPv6 session to carry IPv4 routes (ENH)</span>
              </div>
            </label>
          </div>
        </div>

        <div v-if="error" class="error-banner">{{ error }}</div>

        <div class="form-actions">
          <button type="button" @click="goBack" class="btn-cancel">Cancel</button>
          <button type="submit" class="btn-submit" :disabled="submitting">
            <span v-if="submitting" class="spinner-small"></span>
            Save Changes
          </button>
        </div>
      </form>
    </template>
  </div>
</template>

<style scoped>
.modify-page {
  padding: var(--space-xl) 0;
}

.page-header {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-xl);
}

.btn-back {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-back:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.page-header h2 {
  font-size: 1.25rem;
  font-weight: 600;
}

.loading-state, .error-state {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: var(--space-3xl);
  color: var(--text-tertiary);
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border-color);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.form-container {
  max-width: 800px;
}

.info-banner {
  display: flex;
  gap: var(--space-lg);
  padding: var(--space-md);
  background: var(--accent-light);
  border: 1px solid var(--accent);
  border-radius: var(--radius-md);
  margin-bottom: var(--space-lg);
  font-size: 0.875rem;
  color: var(--accent);
}

.form-section {
  margin-bottom: var(--space-xl);
}

.section-title {
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--text-tertiary);
  margin-bottom: var(--space-md);
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: var(--space-md);
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.form-label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text-secondary);
}

.form-input {
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-primary);
  font-size: 0.875rem;
  transition: border-color var(--transition-fast);
}

.form-input:focus {
  outline: none;
  border-color: var(--accent);
}

.form-input.mono {
  font-family: var(--font-mono);
  font-size: 0.8rem;
}

.toggle-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.toggle-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-md);
  padding: var(--space-md);
  background: var(--bg-secondary);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--transition-fast);
}

.toggle-item:hover {
  background: var(--bg-tertiary);
}

.toggle-item input {
  width: 16px;
  height: 16px;
  margin-top: 5px;
  flex-shrink: 0;
}

.toggle-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.toggle-title {
  font-weight: 500;
  color: var(--text-primary);
}

.toggle-desc {
  font-size: 0.75rem;
  color: var(--text-tertiary);
}

.error-banner {
  padding: var(--space-md);
  background: var(--danger-light);
  border: 1px solid var(--danger);
  border-radius: var(--radius-md);
  color: var(--danger);
  margin-bottom: var(--space-lg);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-md);
  padding-top: var(--space-lg);
  border-top: 1px solid var(--border-color);
}

.btn-cancel {
  padding: var(--space-sm) var(--space-lg);
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  color: var(--text-secondary);
  font-size: 0.875rem;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-cancel:hover {
  border-color: var(--text-tertiary);
  color: var(--text-primary);
}

.btn-submit {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-lg);
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  color: var(--text-inverse);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.btn-submit:hover:not(:disabled) {
  background: var(--accent-hover);
}

.btn-submit:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.spinner-small {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@media (max-width: 640px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
