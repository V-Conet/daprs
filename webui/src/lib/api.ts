import type { MeResponse, NodeActionRequest, NodeAgentConfig, PeeringPayload, RemoveRequest } from '../types'

const apiBase = import.meta.env.VITE_API_BASE?.replace(/\/$/, '') ?? ''

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
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
    throw new Error(message || response.statusText)
  }

  if (response.status === 204) {
    return undefined as T
  }

  return response.json() as Promise<T>
}

export function loginUrl() {
  return `${apiBase}/api/login`
}

export function logout() {
  return requestJson<void>('/api/logout', { method: 'POST' })
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
  return requestJson<boolean>('/api/peering', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function modifyPeering(payload: NodeActionRequest<PeeringPayload>) {
  return requestJson<boolean>('/api/modify', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function removePeering(payload: RemoveRequest) {
  return requestJson<boolean>('/api/remove', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}