import assert from 'node:assert/strict'
import { test } from 'node:test'
import { template, compileFragment, render as renderFragment } from './helpers/vue-ssr.mjs'

const playerRoot = template('src/components/NowPlaying.vue').children.find(node => node.type === 1)
const headerIndex = playerRoot.children.findIndex(node => node.tag === 'header')
assert.ok(headerIndex > 0, 'playback background precedes the header')
const renderBackground = compileFragment(playerRoot.children.slice(0, headerIndex))
const motionSection = template('src/views/SettingsView.vue').children
  .flatMap(function visit(node) { return [node, ...(node.children ?? []).flatMap(visit)] })
  .find(node => node.props?.some(prop => prop.name === 'show' && prop.exp?.content === "activeSettingsSection === 'motion'"))
assert.ok(motionSection, 'motion settings section exists')
const renderSettings = compileFragment([motionSection])

async function render(renderFunction, state) {
  return renderFragment(renderFunction, state, ['CoverBlurBackground', 'HyperBackground', 'ui/EditableRangeValue'])
}

function backgroundState(overrides = {}) {
  return {
    player: { hasPlaybackSession: true, audioLevel: 0.8, beatImpulse: 1 },
    settings: { coverBlurBg: false, dynamicBackground: true, audioReactive: true, coverBlurAmount: 1.5, coverBlurDarken: 0.2 },
    coverUrl: 'https://example.com/cover.jpg',
    accentBgStyle: { background: 'rgb(18,18,18)' },
    shouldRenderDynamicBackground: true,
    extractedColors: undefined,
    paletteResult: null,
    ...overrides,
  }
}

for (const coverBlurBg of [false, true]) {
  for (const dynamicBackground of [false, true]) {
    test(`always renders only the blurred cover with legacy flags ${coverBlurBg}/${dynamicBackground}`, async () => {
      const state = backgroundState()
      Object.assign(state.settings, { coverBlurBg, dynamicBackground })
      const html = await render(renderBackground, state)
      assert.match(html, /class="cover-blur-bg"/)
      assert.match(html, /src="https:\/\/example.com\/cover.jpg"/)
      assert.match(html, /blur\(45px\)/)
      assert.match(html, /rgba\(0,0,0,0\.2\)/)
      assert.doesNotMatch(html, /<canvas|np-ambient-glow|np-scrim/)
    })
  }
}

test('without a playback session only the fallback is rendered', async () => {
  const html = await render(renderBackground, backgroundState({
    player: { hasPlaybackSession: false }, shouldRenderDynamicBackground: false,
  }))
  assert.match(html, /np-bg-solid/)
  assert.doesNotMatch(html, /<img|<canvas|np-ambient-glow/)
})

test('missing cover keeps the fallback without starting a shader', async () => {
  const html = await render(renderBackground, backgroundState({ coverUrl: '' }))
  assert.match(html, /np-bg-solid/)
  assert.doesNotMatch(html, /<img|<canvas|np-ambient-glow/)
})

test('blur settings remain visible when legacy background flags are off', async () => {
  const html = await render(renderSettings, {
    t: key => key, activeSettingsSection: 'motion', isExpanded: () => true,
    onExpandEnter() {}, onExpandAfterEnter() {}, onExpandLeave() {}, onExpandAfterLeave() {},
    advancedLyrics: true, dynamicBackground: false, audioReactive: true, coverBlurBg: false,
    coverBlurAmount: 1.5, coverBlurDarken: 0.2,
  })
  assert.match(html, /settings.blur_amount/)
  assert.match(html, /settings.bg_darken/)
  assert.equal((html.match(/type="range"/g) ?? []).length, 2)
  assert.doesNotMatch(html, /settings.dynamic_bg|settings.cover_blur_desc/)
})
