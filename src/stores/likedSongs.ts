import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { TrackInfo } from '@/stores/player'
import { AUTH_CHANGED_EVENT, useAuthStore } from '@/stores/auth'
import { useRecommendStore } from '@/stores/recommend'
import { useToastStore } from '@/stores/toast'
import i18n from '@/i18n'
import { createLogger } from '@/utils/logger'

const log = createLogger('liked-songs')

export const useLikedSongsStore = defineStore('likedSongs', () => {
  const isLoading = ref(false)
  const isReady = ref(false)
  const likedPlaylistId = ref<number | null>(null)
  const recommend = useRecommendStore()
  const auth = useAuthStore()
  const likedTrackIds = computed(() => new Set(
    [...recommend.likedSongIds].map(id => `netease:${id}`),
  ))

  let loadPromise: Promise<void> | null = null
  let started = false

  function getNeteaseSongId(track?: TrackInfo | null) {
    if (!track?.id.startsWith('netease:')) return null
    const id = Number.parseInt(track.id.slice('netease:'.length), 10)
    return Number.isSafeInteger(id) && id > 0 ? id : null
  }

  async function loadLikedPlaylist() {
    if (loadPromise) return loadPromise
    loadPromise = (async () => {
      isLoading.value = true
      try {
        if (!auth.netease.loggedIn) {
          recommend.likedSongIds.clear()
          return
        }
        await recommend.fetchLikedSongIds()
      } finally {
        isReady.value = true
        isLoading.value = false
        loadPromise = null
      }
    })()
    return loadPromise
  }

  function isTrackLiked(track?: TrackInfo | null) {
    const id = getNeteaseSongId(track)
    return id !== null && recommend.likedSongIds.has(id)
  }

  async function toggleTrack(track?: TrackInfo | null) {
    const songId = getNeteaseSongId(track)
    if (songId === null || !auth.netease.loggedIn) return false

    const shouldLike = !recommend.likedSongIds.has(songId)
    isLoading.value = true
    try {
      const ok = await recommend.toggleLikeSong(songId, shouldLike)
      if (!ok) throw new Error('Netease rejected like operation')
      return true
    } catch (e) {
      log.error('toggleTrack:', e)
      useToastStore().error((i18n.global as any).t('player.like_cloud_sync_failed'))
      await recommend.fetchLikedSongIds()
      return false
    } finally {
      isLoading.value = false
    }
  }

  function handleAuthChanged(event: Event) {
    const platform = (event as CustomEvent<{ platform?: string }>).detail?.platform
    if (platform === 'netease') void loadLikedPlaylist()
  }

  async function start() {
    if (!started) {
      window.addEventListener(AUTH_CHANGED_EVENT, handleAuthChanged)
      started = true
    }
    await loadLikedPlaylist()
  }

  function stop() {
    if (!started) return
    window.removeEventListener(AUTH_CHANGED_EVENT, handleAuthChanged)
    started = false
  }

  return {
    likedPlaylistId,
    likedTrackIds,
    isLoading,
    isReady,
    loadLikedPlaylist,
    isTrackLiked,
    toggleTrack,
    start,
    stop,
  }
})
