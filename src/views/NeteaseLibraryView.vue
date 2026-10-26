<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useAuthStore } from '@/stores/auth'
import { useRecommendStore, type PlaylistInfo } from '@/stores/recommend'
import { useToastStore } from '@/stores/toast'
import M3Dialog from '@/components/ui/M3Dialog.vue'
import M3Input from '@/components/ui/M3Input.vue'
import BilibiliCoverImage from '@/components/BilibiliCoverImage.vue'
import { createLogger } from '@/utils/logger'

defineOptions({ name: 'NeteaseLibraryView' })

type Category = 'playlists' | 'albums' | 'artists'

const router = useRouter()
const { t } = useI18n()
const auth = useAuthStore()
const recommend = useRecommendStore()
const toast = useToastStore()
const log = createLogger('netease-library')

const category = ref<Category>('playlists')
const searchQuery = ref('')
const loading = ref(false)
const mutatingId = ref<number | null>(null)
const createDialogOpen = ref(false)
const renameDialogOpen = ref(false)
const deleteDialogOpen = ref(false)
const playlistName = ref('')
const selectedPlaylist = ref<PlaylistInfo | null>(null)
const nameInput = ref<InstanceType<typeof M3Input>>()

const categories = computed(() => [
  { key: 'playlists' as const, label: t('library.tab_netease_playlists_short'), icon: 'queue_music' },
  { key: 'albums' as const, label: t('library.tab_netease_albums_short'), icon: 'album' },
  { key: 'artists' as const, label: t('library.followed_artists'), icon: 'account_circle' },
])

const query = computed(() => searchQuery.value.trim().toLocaleLowerCase())
const playlists = computed(() => (recommend.userPlaylists.netease || []).filter(item =>
  !query.value || item.name.toLocaleLowerCase().includes(query.value),
))
const albums = computed(() => recommend.userAlbums.filter((item: any) =>
  !query.value || `${item.name} ${item.artist}`.toLocaleLowerCase().includes(query.value),
))
const artists = computed(() => recommend.followedArtists.filter(item =>
  !query.value || item.name.toLocaleLowerCase().includes(query.value),
))

function isEditable(playlist: PlaylistInfo) {
  return !playlist.subscribed && playlist.specialType !== 5
}

async function load(force = false) {
  if (!auth.netease.loggedIn || (loading.value && !force)) return
  loading.value = true
  try {
    await Promise.allSettled([
      recommend.fetchUserPlaylists('netease'),
      recommend.fetchUserAlbums(),
      recommend.fetchFollowedArtists(),
      recommend.fetchLikedSongIds(),
    ])
  } finally {
    loading.value = false
  }
}

function openCreate() {
  selectedPlaylist.value = null
  playlistName.value = ''
  createDialogOpen.value = true
  nextTick(() => nameInput.value?.focus())
}

function openRename(playlist: PlaylistInfo) {
  selectedPlaylist.value = playlist
  playlistName.value = playlist.name
  renameDialogOpen.value = true
  nextTick(() => nameInput.value?.focus())
}

function openDelete(playlist: PlaylistInfo) {
  selectedPlaylist.value = playlist
  deleteDialogOpen.value = true
}

async function createPlaylist() {
  const name = playlistName.value.trim()
  if (!name) return
  try {
    await recommend.createCloudPlaylist(name)
    createDialogOpen.value = false
    toast.success(t('library.cloud_playlist_created'))
  } catch (error) {
    log.error('create playlist:', error)
    toast.error(String(error))
  }
}

async function renamePlaylist() {
  const playlist = selectedPlaylist.value
  const name = playlistName.value.trim()
  if (!playlist || !name) return
  try {
    await recommend.renameCloudPlaylist(Number(playlist.id), name)
    renameDialogOpen.value = false
    toast.success(t('library.cloud_playlist_renamed'))
  } catch (error) {
    log.error('rename playlist:', error)
    toast.error(String(error))
  }
}

async function deletePlaylist() {
  const playlist = selectedPlaylist.value
  if (!playlist) return
  try {
    await recommend.deleteCloudPlaylist(Number(playlist.id))
    deleteDialogOpen.value = false
    toast.success(t('library.cloud_playlist_deleted'))
  } catch (error) {
    log.error('delete playlist:', error)
    toast.error(String(error))
  }
}

async function unsubscribePlaylist(playlist: PlaylistInfo) {
  const id = Number(playlist.id)
  mutatingId.value = id
  try {
    await recommend.setPlaylistSubscribed(id, false)
    toast.success(t('library.playlist_unsubscribed'))
  } catch (error) {
    log.error('unsubscribe playlist:', error)
    toast.error(String(error))
  } finally {
    mutatingId.value = null
  }
}

async function unfollowArtist(artistId: number) {
  mutatingId.value = artistId
  try {
    await recommend.setArtistFollowed(artistId, false)
    toast.success(t('library.artist_unfollowed'))
  } catch (error) {
    log.error('unfollow artist:', error)
    toast.error(String(error))
  } finally {
    mutatingId.value = null
  }
}

watch(
  () => auth.netease.loggedIn,
  loggedIn => {
    if (loggedIn) void load()
  },
  { immediate: true },
)
</script>

<template>
  <div class="cloud-library">
    <header class="library-header">
      <div>
        <h1>{{ t('library.title') }}</h1>
        <p>{{ t('library.netease_cloud_library') }}</p>
      </div>
      <button
        v-if="auth.netease.loggedIn && category === 'playlists'"
        class="icon-button primary"
        :title="t('library.create_playlist')"
        @click="openCreate"
      >
        <span class="material-symbols-rounded">playlist_add</span>
      </button>
    </header>

    <div v-if="!auth.netease.loggedIn" class="empty-state">
      <span class="material-symbols-rounded">cloud_off</span>
      <h2>{{ t('library.netease_login_required') }}</h2>
      <button class="primary-action" @click="router.push({ name: 'settings' })">
        {{ t('settings.sign_in') }}
      </button>
    </div>

    <template v-else>
      <div class="toolbar">
        <div class="segmented-control">
          <button
            v-for="item in categories"
            :key="item.key"
            :class="{ active: category === item.key }"
            @click="category = item.key"
          >
            <span class="material-symbols-rounded">{{ item.icon }}</span>
            <span>{{ item.label }}</span>
          </button>
        </div>
        <label class="search-box">
          <span class="material-symbols-rounded">search</span>
          <input v-model="searchQuery" data-shortcut-search type="search" :placeholder="t('library.tab_search_hint')" />
          <button v-if="searchQuery" :title="t('common.clear')" @click.prevent="searchQuery = ''">
            <span class="material-symbols-rounded">close</span>
          </button>
        </label>
      </div>

      <div v-if="loading" class="loading-row">
        <span class="material-symbols-rounded spinning">progress_activity</span>
      </div>

      <div v-else-if="category === 'playlists'" class="item-list">
        <div
          v-for="playlist in playlists"
          :key="playlist.id"
          class="library-item"
          role="button"
          tabindex="0"
          @click="router.push({ name: 'netease-playlist', params: { id: playlist.id } })"
          @keydown.enter="router.push({ name: 'netease-playlist', params: { id: playlist.id } })"
        >
          <div class="cover">
            <BilibiliCoverImage v-if="playlist.coverUrl" :src="playlist.coverUrl">
              <span class="material-symbols-rounded">queue_music</span>
            </BilibiliCoverImage>
            <span v-else class="material-symbols-rounded">queue_music</span>
          </div>
          <div class="item-copy">
            <strong>{{ playlist.name }}</strong>
            <span>{{ t('library.track_count', { count: playlist.trackCount }) }}</span>
          </div>
          <div class="item-actions">
            <button v-if="playlist.subscribed" :disabled="mutatingId === Number(playlist.id)" :title="t('library.unsubscribe_playlist')" @click.stop="unsubscribePlaylist(playlist)">
              <span class="material-symbols-rounded filled">bookmark_remove</span>
            </button>
            <template v-else-if="isEditable(playlist)">
              <button :title="t('library.rename_playlist')" @click.stop="openRename(playlist)"><span class="material-symbols-rounded">edit</span></button>
              <button class="danger" :title="t('library.delete_playlist')" @click.stop="openDelete(playlist)"><span class="material-symbols-rounded">delete</span></button>
            </template>
            <span v-else class="material-symbols-rounded chevron">chevron_right</span>
          </div>
        </div>
        <div v-if="playlists.length === 0" class="empty-state compact"><span class="material-symbols-rounded">queue_music</span><p>{{ t('explore.no_playlists') }}</p></div>
      </div>

      <div v-else-if="category === 'albums'" class="item-list">
        <div v-for="album in albums" :key="album.id" class="library-item" role="button" tabindex="0" @click="router.push({ name: 'netease-album', params: { id: album.id } })">
          <div class="cover">
            <BilibiliCoverImage v-if="album.coverUrl" :src="album.coverUrl">
              <span class="material-symbols-rounded">album</span>
            </BilibiliCoverImage>
            <span v-else class="material-symbols-rounded">album</span>
          </div>
          <div class="item-copy"><strong>{{ album.name }}</strong><span>{{ album.artist }}</span></div>
          <span class="material-symbols-rounded chevron">chevron_right</span>
        </div>
        <div v-if="albums.length === 0" class="empty-state compact"><span class="material-symbols-rounded">album</span><p>{{ t('library.empty_title', { type: t('library.albums') }) }}</p></div>
      </div>

      <div v-else class="item-list">
        <div v-for="artist in artists" :key="artist.id" class="library-item" role="button" tabindex="0" @click="router.push({ name: 'netease-artist', params: { id: artist.id } })">
          <div class="cover round">
            <BilibiliCoverImage v-if="artist.coverUrl" :src="artist.coverUrl">
              <span class="material-symbols-rounded">account_circle</span>
            </BilibiliCoverImage>
            <span v-else class="material-symbols-rounded">account_circle</span>
          </div>
          <div class="item-copy"><strong>{{ artist.name }}</strong><span>{{ t('player.track_count', { count: artist.trackCount }) }}</span></div>
          <button class="item-action" :disabled="mutatingId === artist.id" :title="t('library.unfollow_artist')" @click.stop="unfollowArtist(artist.id)"><span class="material-symbols-rounded filled">person_remove</span></button>
        </div>
        <div v-if="artists.length === 0" class="empty-state compact"><span class="material-symbols-rounded">account_circle</span><p>{{ t('library.no_followed_artists') }}</p></div>
      </div>
    </template>

    <M3Dialog v-model:open="createDialogOpen" :title="t('library.create_playlist')" icon="playlist_add" :confirm-text="t('common.confirm')" :confirm-disabled="!playlistName.trim()" @confirm="createPlaylist">
      <M3Input ref="nameInput" v-model="playlistName" :placeholder="t('library.playlist_name_placeholder')" :maxlength="50" @enter="createPlaylist" />
    </M3Dialog>
    <M3Dialog v-model:open="renameDialogOpen" :title="t('library.rename_playlist')" icon="edit" :confirm-text="t('common.save')" :confirm-disabled="!playlistName.trim()" @confirm="renamePlaylist">
      <M3Input ref="nameInput" v-model="playlistName" :placeholder="t('library.rename_placeholder')" :maxlength="50" @enter="renamePlaylist" />
    </M3Dialog>
    <M3Dialog v-model:open="deleteDialogOpen" :title="t('library.delete_confirm_title')" icon="delete" :confirm-text="t('common.delete')" confirm-danger @confirm="deletePlaylist">
      <p>{{ t('library.delete_confirm_msg', { name: selectedPlaylist?.name || '' }) }}</p>
    </M3Dialog>
  </div>
</template>

<style scoped lang="scss">
.cloud-library { padding: 20px 28px 36px; }
.library-header { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 18px; }
.library-header h1 { margin: 0; font-size: 28px; }
.library-header p { margin: 4px 0 0; color: var(--md-on-surface-variant); font-size: 13px; }
.toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 18px; }
.segmented-control { display: flex; padding: 3px; gap: 2px; background: var(--md-surface-container); border-radius: var(--radius-lg); }
.segmented-control button { min-height: 36px; padding: 0 14px; display: flex; align-items: center; gap: 6px; border-radius: var(--radius-md); color: var(--md-on-surface-variant); }
.segmented-control button.active { background: var(--md-secondary-container); color: var(--md-on-secondary-container); font-weight: 700; }
.segmented-control .material-symbols-rounded { font-size: 18px; }
.search-box { width: min(320px, 40vw); height: 40px; display: flex; align-items: center; gap: 8px; padding: 0 12px; background: var(--md-surface-container); border-radius: var(--radius-lg); }
.search-box input { min-width: 0; flex: 1; border: 0; outline: 0; background: transparent; color: inherit; }
.search-box button { display: grid; place-items: center; }
.search-box .material-symbols-rounded { font-size: 19px; }
.item-list { display: flex; flex-direction: column; gap: 2px; }
.library-item { min-height: 68px; display: flex; align-items: center; gap: 13px; padding: 8px 10px; border-radius: var(--radius-md); cursor: pointer; }
.library-item:hover { background: var(--md-surface-container); }
.cover { width: 52px; height: 52px; flex: 0 0 52px; overflow: hidden; border-radius: var(--radius-sm); display: grid; place-items: center; background: var(--md-surface-container-high); color: var(--md-primary); }
.cover.round { border-radius: 50%; }
.cover img { width: 100%; height: 100%; object-fit: cover; }
.item-copy { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 4px; }
.item-copy strong, .item-copy span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.item-copy strong { font-size: 14px; }
.item-copy span { color: var(--md-on-surface-variant); font-size: 12px; }
.item-actions { display: flex; align-items: center; gap: 2px; }
.item-actions button, .item-action, .icon-button { width: 40px; height: 40px; display: grid; place-items: center; border-radius: 50%; color: var(--md-on-surface-variant); }
.item-actions button:hover, .item-action:hover, .icon-button:hover { background: var(--md-surface-container-high); }
.item-actions .danger { color: var(--md-error); }
.icon-button.primary { color: var(--md-primary); background: var(--md-primary-container); }
.chevron { opacity: .35; }
.empty-state { min-height: 45vh; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; text-align: center; color: var(--md-on-surface-variant); }
.empty-state > .material-symbols-rounded { font-size: 44px; opacity: .5; }
.empty-state.compact { min-height: 260px; }
.primary-action { min-height: 40px; padding: 0 18px; border-radius: var(--radius-full); color: var(--md-on-primary); background: var(--md-primary); font-weight: 700; }
.loading-row { min-height: 180px; display: grid; place-items: center; }
.spinning { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 720px) {
  .cloud-library { padding: 16px; }
  .toolbar { align-items: stretch; flex-direction: column; }
  .segmented-control { display: grid; grid-template-columns: repeat(3, 1fr); }
  .segmented-control button { justify-content: center; padding: 0 6px; }
  .search-box { width: 100%; }
}
</style>
