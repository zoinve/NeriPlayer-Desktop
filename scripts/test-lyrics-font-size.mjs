import assert from 'node:assert/strict'
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { pathToFileURL } from 'node:url'
import { spawnSync } from 'node:child_process'
import { test } from 'node:test'
import { compileStyleAsync, parse } from 'vue/compiler-sfc'
import { root, template, compileFragment, render } from './helpers/vue-ssr.mjs'

const descendants = node => [node, ...(node.children ?? []).flatMap(descendants)]
const settingsNodes = descendants(template('src/views/SettingsView.vue'))
const lyricsSection = settingsNodes.find(node => node.props?.some(
  prop => prop.name === 'show' && prop.exp?.content === "activeSettingsSection === 'lyrics'",
))

for (const scale of [0.5, 1, 1.5]) {
  test(`lyrics settings expose the playback font size at ${scale * 100}% without expanding`, async () => {
    const html = await render(compileFragment([lyricsSection]), {
      t: key => key, activeSettingsSection: 'lyrics', isExpanded: () => false,
      onExpandEnter() {}, onExpandAfterEnter() {}, onExpandLeave() {}, onExpandAfterLeave() {},
      lyricFontScale: scale, showTranslation: true, lyricBlur: true,
    }, ['ui/EditableRangeValue'])
    assert.match(html, /settings.lyric_font_size/)
    assert.match(html, new RegExp(`${scale * 100}%`))
    assert.match(html, new RegExp(`<input[^>]*type="range"[^>]*min="0.5"[^>]*max="1.5"[^>]*step="0.05"[^>]*value="${scale}"`))
  })
}

test('the playback font size is not duplicated in personalization', () => {
  const section = settingsNodes.find(node => node.props?.some(
    prop => prop.name === 'show' && prop.exp?.content === "activeSettingsSection === 'personalization'",
  ))
  const fontControls = descendants(section).filter(node => node.props?.some(
    prop => prop.name === 'model' && prop.exp?.content === 'lyricFontScale',
  ))
  assert.equal(fontControls.length, 0)
})

test('playback font controls use the same supported limits as settings', async () => {
  const fontView = descendants(template('src/components/NowPlaying.vue')).find(node =>
    node.tag === 'template' && node.props?.some(prop => prop.exp?.content === "moreSheetView === 'fontsize'"),
  )
  const html = await render(compileFragment(fontView.children), {
    t: key => key, goBackToMain() {}, settings: { lyricFontScale: 1 },
  }, ['ui/EditableRangeValue'])
  assert.match(html, /<input[^>]*type="range"[^>]*min="0.5"[^>]*max="1.5"[^>]*step="0.05"/)
})

const browser = [process.env.LYRICS_TEST_BROWSER,
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  '/usr/bin/chromium', '/usr/bin/chromium-browser', '/usr/bin/google-chrome',
].find(path => path && existsSync(path))

test('rendered lyric font size stays identical across window and fullscreen viewports', {
  skip: browser ? false : 'Set LYRICS_TEST_BROWSER to a Chromium browser to run the CSS regression test',
}, async t => {
  const filename = join(root, 'src/components/LyricsView.vue')
  const { descriptor } = parse(readFileSync(filename, 'utf8'), { filename })
  const style = await compileStyleAsync({
    source: descriptor.styles[0].content, filename, id: 'font-test',
    preprocessLang: descriptor.styles[0].lang,
  })
  assert.deepEqual(style.errors, [])
  const host = template('src/components/LyricsView.vue').children.find(node => node.tag === 'div')
  const renderHost = compileFragment([host])
  const cases = []
  for (const scale of [0.5, 1, 1.5]) {
    const html = await render(renderHost, {
      settings: { lyricFontScale: scale }, isLayoutReady: true, onLyricContextMenu() {},
    })
    for (const [width, height] of [[1280, 800], [1920, 1080], [800, 600]]) {
      cases.push({ scale, width, height, html })
    }
  }
  const tempBase = process.platform === 'win32'
    ? join(process.env.LOCALAPPDATA, 'Temp/opencode') : tmpdir()
  const directory = mkdtempSync(join(tempBase, 'lyrics-font-'))
  try {
    const page = join(directory, 'test.html')
    writeFileSync(page, `<pre id="result"></pre><script>
      const cases = ${JSON.stringify(cases)};
      const css = ${JSON.stringify(style.code)};
      Promise.all(cases.map(item => new Promise(resolve => {
        const frame = document.createElement('iframe');
        frame.style.cssText = 'position:absolute;left:-10000px;border:0';
        frame.width = item.width; frame.height = item.height;
        frame.onload = () => {
          const doc = frame.contentDocument;
          const host = doc.querySelector('.lyrics-scroll');
          const line = doc.createElement('span');
          line.style.fontSize = 'var(--amll-lp-font-size)';
          host.append(line);
          resolve({ scale: item.scale, width: frame.contentWindow.innerWidth,
            height: frame.contentWindow.innerHeight,
            fontSize: parseFloat(frame.contentWindow.getComputedStyle(line).fontSize) });
        };
        frame.srcdoc = '<style>' + css + '</style>' + item.html;
        document.body.append(frame);
      }))).then(results => document.querySelector('#result').textContent = JSON.stringify(results));
    </script>`)
    const result = spawnSync(browser, [
      '--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
      `--user-data-dir=${join(directory, 'profile')}`, '--dump-dom', '--virtual-time-budget=3000',
      pathToFileURL(page).href,
    ], { encoding: 'utf8', timeout: 30000, maxBuffer: 4 * 1024 * 1024 })
    assert.ifError(result.error)
    assert.equal(result.status, 0, result.stderr)
    const match = result.stdout.match(/<pre id="result">([^<]+)<\/pre>/)
    assert.ok(match, 'browser must finish measuring all viewport cases')
    const measurements = JSON.parse(match[1])
    t.diagnostic(JSON.stringify(measurements))
    assert.equal(measurements.length, cases.length)
    for (const [index, measurement] of measurements.entries()) {
      assert.equal(measurement.width, cases[index].width)
      assert.equal(measurement.height, cases[index].height)
      assert.equal(measurement.fontSize, 40 * measurement.scale,
        `scale ${measurement.scale} at ${measurement.width}x${measurement.height}`)
    }
  } finally {
    rmSync(directory, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
  }
})
