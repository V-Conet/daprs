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

// 表单数据
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

    // 获取现有配置
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
  <div>
    <div class="d-flex align-items-center mb-4">
      <button @click="goBack" class="btn btn-outline-secondary btn-sm me-3">
        <i class="bi bi-arrow-left"></i>
      </button>
      <h2 class="mb-0">
        <i class="bi bi-pencil me-2"></i>
        Modify Peer: {{ nodeName }} / AS{{ targetAsn }}
      </h2>
    </div>

    <div v-if="loading" class="text-center py-5">
      <div class="spinner-border text-primary" role="status"></div>
    </div>

    <div v-else-if="error && !node" class="alert alert-danger">{{ error }}</div>

    <template v-else-if="node">
      <div class="card shadow-sm">
        <div class="card-body">
          <form @submit.prevent="submitForm">
            <div class="alert alert-info">
              <strong>Node:</strong> {{ nodeName }} |
              <strong>Target ASN:</strong> AS{{ targetAsn }}
            </div>

            <h5 class="mb-3"><i class="bi bi-shield-lock me-1"></i> WireGuard Configuration</h5>

            <div class="row g-3 mb-4">
              <div class="col-md-6">
                <label class="form-label">Endpoint *</label>
                <input v-model="form.endpoint" type="text" class="form-control font-monospace" placeholder="e.g., example.com:51820" required />
              </div>
              <div class="col-md-6">
                <label class="form-label">Public Key *</label>
                <input v-model="form.pubkey" type="text" class="form-control font-monospace" placeholder="WireGuard public key" required />
              </div>
            </div>

            <div class="row g-3 mb-4">
              <div class="col-md-4">
                <label class="form-label">Pre-Shared Key</label>
                <input v-model="form.psk" type="text" class="form-control font-monospace" placeholder="Optional PSK" />
              </div>
              <div class="col-md-4">
                <label class="form-label">MTU</label>
                <input v-model.number="form.mtu" type="number" class="form-control" placeholder="1420" min="576" max="9000" />
              </div>
              <div class="col-md-4">
                <label class="form-label">Port</label>
                <input v-model.number="form.custom_port" type="number" class="form-control" :placeholder="`Default: ${defaultPort}`" min="1024" max="65535" />
              </div>
            </div>

            <h5 class="mb-3"><i class="bi bi-diagram-3 me-1"></i> IP Address Configuration</h5>

            <div class="row g-3 mb-4">
              <div class="col-md-6">
                <label class="form-label">Tunnel IPv4</label>
                <input v-model="form.v4" type="text" class="form-control font-monospace" placeholder="DN42 IPv4 address" />
              </div>
              <div class="col-md-6">
                <label class="form-label">Tunnel IPv6</label>
                <input v-model="form.v6" type="text" class="form-control font-monospace" placeholder="DN42 IPv6 or Link-Local address" />
              </div>
            </div>

            <h5 class="mb-3"><i class="bi bi-sliders me-1"></i> BGP Extensions</h5>

            <div class="card border mb-4">
              <div class="card-body">
                <div class="form-check form-switch mb-2">
                  <input v-model="form.is_mhp" class="form-check-input" type="checkbox" id="mpbgp" @change="onMhpChange(form.is_mhp)" />
                  <label class="form-check-label" for="mpbgp">
                    <strong>Multiprotocol BGP (MP-BGP)</strong>
                    <span class="d-block form-text">Single BGP session for both IPv4 and IPv6 routes</span>
                  </label>
                </div>
                <div class="form-check form-switch">
                  <input v-model="form.is_nhp" class="form-check-input" type="checkbox" id="enexthop" :disabled="!form.is_mhp" />
                  <label class="form-check-label" for="enexthop">
                    <strong>Extended Next Hop (IPv6 session)</strong>
                    <span class="d-block form-text">Use IPv6 session to carry IPv4 routes (ENH)</span>
                  </label>
                </div>
              </div>
            </div>

            <div v-if="error" class="alert alert-danger">{{ error }}</div>

            <div class="d-flex justify-content-end gap-2">
              <button type="button" @click="goBack" class="btn btn-outline-secondary">Cancel</button>
              <button type="submit" class="btn btn-primary" :disabled="submitting">
                <span v-if="submitting" class="spinner-border spinner-border-sm me-1"></span>
                Save Changes
              </button>
            </div>
          </form>
        </div>
      </div>
    </template>
  </div>
</template>
