import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import type { TrackInfo } from './player'
import { createLogger } from '@/utils/logger'

export type NeteaseHistoryMode = 'recent' | 'ranking'
export type NeteaseRankingPeriod = 'week' | 'all'

export interface NeteaseHistoryEntry {
  track: TrackInfo
  playedAt: number
  playCount: number
  score: number
}

const log = createLogger('netease-history')

function mapSong(raw: any): TrackInfo | null {
  const id = Number(raw?.id) || 0
  const title = String(raw?.name || '').trim()
  if (!id || !title) return null
  const artists = Array.isArray(raw?.ar) ? raw.ar : Array.isArray(raw?.artists) ? raw.artists : []
  const album = raw?.al || raw?.album || {}
  return {
    id: `netease:${id}`,
    title,
    artist: artists.map((artist: any) => String(artist?.name || '')).filter(Boolean).join(' / '),
    album: String(album?.name || ''),
    durationMs: Number(raw?.dt ?? raw?.duration ?? 0) || 0,
    coverUrl: String(album?.picUrl || '').replace(/^http:\/\//, 'https://'),
    audioUrl: '',
    source: 'netease',
  }
}

function parseRecent(raw: any): NeteaseHistoryEntry[] {
  const list = Array.isArray(raw?.data?.list) ? raw.data.list : []
  return list.flatMap((item: any) => {
    if (item?.banned || (item?.resourceType && String(item.resourceType).toUpperCase() !== 'SONG')) return []
    const track = mapSong(item?.data)
    return track ? [{ track, playedAt: Number(item?.playTime) || 0, playCount: 0, score: 0 }] : []
  }).sort((left: NeteaseHistoryEntry, right: NeteaseHistoryEntry) => right.playedAt - left.playedAt)
}

function parseRanking(raw: any, period: NeteaseRankingPeriod): NeteaseHistoryEntry[] {
  const list = raw?.[period === 'week' ? 'weekData' : 'allData']
  if (!Array.isArray(list)) return []
  return list.flatMap((item: any) => {
    const track = mapSong(item?.song)
    return track ? [{
      track,
      playedAt: 0,
      playCount: Math.max(0, Number(item?.playCount) || 0),
      score: Math.max(0, Number(item?.score) || 0),
    }] : []
  })
}

export const useNeteaseHistoryStore = defineStore('neteaseHistory', () => {
  const mode = ref<NeteaseHistoryMode>('recent')
  const rankingPeriod = ref<NeteaseRankingPeriod>('week')
  const recentEntries = ref<NeteaseHistoryEntry[]>([])
  const rankingEntries = ref<Record<NeteaseRankingPeriod, NeteaseHistoryEntry[]>>({ week: [], all: [] })
  const loading = ref(false)
  const error = ref<string | null>(null)
  let generation = 0

  async function refresh() {
    const currentGeneration = ++generation
    loading.value = true
    error.value = null
    try {
      if (mode.value === 'recent') {
        const raw = await invoke<any>('netease_get_recent_play_songs', { limit: 100 })
        if (currentGeneration === generation) recentEntries.value = parseRecent(raw)
      } else {
        const period = rankingPeriod.value
        const raw = await invoke<any>('netease_get_user_record', { period })
        if (currentGeneration === generation) {
          rankingEntries.value = { ...rankingEntries.value, [period]: parseRanking(raw, period) }
        }
      }
    } catch (cause) {
      if (currentGeneration === generation) error.value = String(cause)
      log.warn('load server playback history failed:', cause)
    } finally {
      if (currentGeneration === generation) loading.value = false
    }
  }

  function reset() {
    generation++
    mode.value = 'recent'
    rankingPeriod.value = 'week'
    recentEntries.value = []
    rankingEntries.value = { week: [], all: [] }
    loading.value = false
    error.value = null
  }

  return { mode, rankingPeriod, recentEntries, rankingEntries, loading, error, refresh, reset }
})
