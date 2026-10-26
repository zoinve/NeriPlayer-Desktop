<script setup lang="ts">
import { ref, computed } from 'vue'

const props = withDefaults(defineProps<{
  progress: number
  activeColor?: string
  inactiveColor?: string
}>(), {
  activeColor: '#fff',
  inactiveColor: 'rgba(255,255,255,0.35)',
})

const emit = defineEmits<{
  seek: [progress: number]
  preview: [progress: number]
  'preview-end': []
}>()

const containerRef = ref<HTMLDivElement>()
const isDragging = ref(false)
const dragProgress = ref(0)

const currentProgress = computed(() => isDragging.value ? dragProgress.value : props.progress)

function updateDragProgress(e: PointerEvent): boolean {
  const el = containerRef.value
  if (!el) return false
  const rect = el.getBoundingClientRect()
  dragProgress.value = clamp01((e.clientX - rect.left) / rect.width)
  return true
}

// setPointerCapture 模式（与 MiniPlayer 进度条一致）：
// 指针离开组件后事件仍派发到本元素，无需在 document 上挂全局监听
function handlePointerDown(e: PointerEvent) {
  e.preventDefault()
  e.stopPropagation()
  if (!updateDragProgress(e)) return
  isDragging.value = true
  containerRef.value?.setPointerCapture(e.pointerId)
  emit('preview', dragProgress.value)
}

function handlePointerMove(e: PointerEvent) {
  if (!isDragging.value) return
  if (!updateDragProgress(e)) return
  emit('preview', dragProgress.value)
}

function handlePointerUp(e: PointerEvent) {
  if (!isDragging.value) return
  updateDragProgress(e)
  const p = dragProgress.value
  isDragging.value = false
  containerRef.value?.releasePointerCapture(e.pointerId)
  emit('seek', p)
  emit('preview-end')
}

function clamp01(v: number) { return Math.max(0, Math.min(1, v)) }
</script>

<template>
  <div
    ref="containerRef"
    class="waveform-container"
    :style="{ '--wave-active-color': activeColor, '--wave-inactive-color': inactiveColor }"
    @pointerdown="handlePointerDown"
    @pointermove="handlePointerMove"
    @pointerup="handlePointerUp"
    @pointercancel="handlePointerUp"
  >
    <div class="line-track" />
    <div class="line-active" :style="{ width: `${currentProgress * 100}%` }" />
    <div
      class="thumb"
      :class="{ dragging: isDragging }"
      :style="{ left: `${currentProgress * 100}%` }"
    />
  </div>
</template>

<style scoped>
.waveform-container {
  position: relative;
  width: 100%;
  height: 8px;
  cursor: pointer;
  touch-action: none;
}

/* 扩大点击热区的透明伪元素 */
.waveform-container::before {
  content: '';
  position: absolute;
  top: -12px;
  bottom: -12px;
  left: 0;
  right: 0;
}

.line-track,
.line-active {
  position: absolute;
  top: 50%;
  left: 0;
  height: 3px;
  border-radius: 999px;
  transform: translateY(-50%);
}

.line-track {
  width: 100%;
  background: var(--wave-inactive-color, rgba(255,255,255,0.35));
}

.line-active {
  background: var(--wave-active-color, #fff);
}

.thumb {
  position: absolute;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--waveform-thumb-color, #fff);
  top: 50%;
  transform: translate(-50%, -50%);
  transition: width 150ms, height 150ms,
              background 0.72s cubic-bezier(0.22, 1, 0.36, 1),
              box-shadow 0.72s cubic-bezier(0.22, 1, 0.36, 1);
  box-shadow: 0 0 4px rgba(255, 255, 255, 0.3);
  pointer-events: none;
  z-index: 1;
  will-change: left;
}

.thumb.dragging {
  width: 12px;
  height: 12px;
}
</style>
