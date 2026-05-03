import type { MeResponse, NodeActionRequest, NodeAgentConfig, PeeringPayload, RemoveRequest } from '../types'

const apiBase = import.meta.env.VITE_API_BASE?.replace(/\/$/, '') ?? ''

async function request(path: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(`${apiBase}${path}`, {
    credentials: 'include',
    ...init,
    headers: {
      Accept: 'application/json',
      ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
      ...(init?.headers ?? {}),
    },
  })

  if (!response.ok) {
    const message = await response.text().catch(() => response.statusText)
    throw new Error(`${response.status} ${message || response.statusText}`.trim())
  }

  return response
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await request(path, init)

  if (response.status === 204) {
    return undefined as T
  }

  const contentLength = response.headers.get('content-length')
  if (contentLength === '0') {
    return undefined as T
  }

  const contentType = response.headers.get('content-type') || ''
  if (!contentType.toLowerCase().includes('application/json')) {
    return undefined as T
  }

  return response.json() as Promise<T>
}

async function requestVoid(path: string, init?: RequestInit): Promise<void> {
  await request(path, init)
}

export function loginUrl() {
  return `${apiBase}/api/login`
}

export function logout() {
  return requestVoid('/api/logout', { method: 'POST' })
}

export function fetchMe() {
  return requestJson<MeResponse>('/api/me')
}

export function fetchNodes() {
  return requestJson<Record<string, NodeAgentConfig>>('/api/nodes')
}

export function fetchPeers() {
  return requestJson<Array<NodeActionRequest<PeeringPayload>>>('/api/peers')
}

export function createPeering(payload: NodeActionRequest<PeeringPayload>) {
  return requestVoid('/api/peering', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function modifyPeering(payload: NodeActionRequest<PeeringPayload>) {
  return requestVoid('/api/modify', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function removePeering(payload: RemoveRequest) {
  return requestVoid('/api/remove', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function deletePeeringQueue(node: string) {
  return requestVoid(`/api/peering/${encodeURIComponent(node)}`, { method: 'DELETE' })
}