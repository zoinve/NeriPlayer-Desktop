import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createSSRApp } from 'vue'
import { compileScript, compileTemplate, parse } from 'vue/compiler-sfc'
import { renderToString } from 'vue/server-renderer'
import { createI18n } from 'vue-i18n'
import ts from 'typescript'

const require = createRequire(import.meta.url)
export const root = fileURLToPath(new URL('../../', import.meta.url))
const cache = new Map()

// Compile real Vue components for Node, without mounting browser-only effects.
function evaluate(source, filename) {
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText
  const module = { exports: {} }
  const localRequire = id => {
    if (!id.startsWith('.') && !id.startsWith('@/')) return require(id)
    const path = id.startsWith('@/') ? resolve(root, 'src', id.slice(2)) : resolve(dirname(filename), id)
    if (path.endsWith('?raw')) return { default: readFileSync(path.slice(0, -4), 'utf8') }
    return load(path.endsWith('.vue') ? path : `${path}.ts`)
  }
  new Function('require', 'module', 'exports', compiled)(localRequire, module, module.exports)
  return module.exports
}

function load(filename) {
  if (cache.has(filename)) return cache.get(filename)
  const source = readFileSync(filename, 'utf8')
  const code = filename.endsWith('.vue')
    ? compileScript(parse(source, { filename }).descriptor, { id: filename, inlineTemplate: true }).content
    : source
  const exports = evaluate(code, filename)
  cache.set(filename, exports)
  return exports
}

export function template(filename) {
  return parse(readFileSync(resolve(root, filename), 'utf8')).descriptor.template.ast
}

export function compileFragment(nodes) {
  const { code, errors } = compileTemplate({
    source: nodes.map(node => node.loc.source).join('\n'),
    filename: 'fragment-test.vue',
    id: 'fragment-test',
  })
  assert.deepEqual(errors, [])
  return evaluate(code, resolve(root, 'fragment-test.vue')).render
}

export async function render(renderFunction, state, components = []) {
  const app = createSSRApp({ render: renderFunction, data: () => state })
  for (const name of components) {
    app.component(name.split('/').at(-1), load(resolve(root, `src/components/${name}.vue`)).default)
  }
  app.use(createI18n({ legacy: false, locale: 'en', missingWarn: false, fallbackWarn: false }))
  return renderToString(app)
}
