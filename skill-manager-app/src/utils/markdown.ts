import DOMPurify from 'dompurify'
import hljs from 'highlight.js/lib/core'
import bash from 'highlight.js/lib/languages/bash'
import css from 'highlight.js/lib/languages/css'
import javascript from 'highlight.js/lib/languages/javascript'
import json from 'highlight.js/lib/languages/json'
import markdownLanguage from 'highlight.js/lib/languages/markdown'
import python from 'highlight.js/lib/languages/python'
import rust from 'highlight.js/lib/languages/rust'
import sql from 'highlight.js/lib/languages/sql'
import typescript from 'highlight.js/lib/languages/typescript'
import xml from 'highlight.js/lib/languages/xml'
import yaml from 'highlight.js/lib/languages/yaml'
import MarkdownIt from 'markdown-it'

hljs.registerLanguage('bash', bash)
hljs.registerLanguage('css', css)
hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('json', json)
hljs.registerLanguage('markdown', markdownLanguage)
hljs.registerLanguage('python', python)
hljs.registerLanguage('rust', rust)
hljs.registerLanguage('shell', bash)
hljs.registerLanguage('sql', sql)
hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('xml', xml)
hljs.registerLanguage('yaml', yaml)

const markdown = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  highlight(code, lang) {
    if (lang && hljs.getLanguage(lang)) {
      return hljs.highlight(code, { language: lang, ignoreIllegals: true }).value
    }

    return hljs.highlightAuto(code).value
  },
})

const MARKDOWN_TAGS = [
  'a',
  'blockquote',
  'br',
  'code',
  'em',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'hr',
  'li',
  'ol',
  'p',
  'pre',
  'span',
  'strong',
  'table',
  'tbody',
  'td',
  'th',
  'thead',
  'tr',
  'ul',
]

const MARKDOWN_ATTR = ['class', 'href', 'rel', 'target']
const MAX_RENDER_CACHE_ENTRIES = 50
const renderCache = new Map<string, string>()

export function renderMarkdown(source: string): string {
  return cachedRender(`markdown:${source}`, () =>
    DOMPurify.sanitize(markdown.render(source), {
      ALLOWED_TAGS: MARKDOWN_TAGS,
      ALLOWED_ATTR: MARKDOWN_ATTR,
      ALLOW_DATA_ATTR: false,
    }),
  )
}

export function renderMarkdownSource(source: string): string {
  return renderSource(source, 'markdown')
}

export function renderSource(source: string, language: string): string {
  return cachedRender(`source:${language}:${source}`, () => {
    const highlighted = hljs.getLanguage(language)
      ? hljs.highlight(source, { language, ignoreIllegals: true }).value
      : hljs.highlightAuto(source).value

    return DOMPurify.sanitize(highlighted, {
      ALLOWED_TAGS: ['span'],
      ALLOWED_ATTR: ['class'],
      ALLOW_DATA_ATTR: false,
    })
  })
}

function cachedRender(key: string, render: () => string): string {
  const cached = renderCache.get(key)
  if (cached !== undefined) return cached

  const rendered = render()
  renderCache.set(key, rendered)
  if (renderCache.size > MAX_RENDER_CACHE_ENTRIES) {
    const oldestKey = renderCache.keys().next().value
    if (oldestKey !== undefined) renderCache.delete(oldestKey)
  }
  return rendered
}
