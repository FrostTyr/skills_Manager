import { describe, expect, it, vi } from 'vitest'

vi.mock('dompurify', () => ({
  default: {
    sanitize: (source: string) => source,
  },
}))

import { renderMarkdown, renderSource } from './markdown'

describe('markdown rendering', () => {
  it('removes executable html from skill content', () => {
    const result = renderMarkdown('# Safe\n\n<script>alert("xss")</script>')

    expect(result).toContain('<h1>Safe</h1>')
    expect(result).not.toContain('<script')
  })

  it('highlights registered source languages', () => {
    const result = renderSource('const answer: number = 42', 'typescript')

    expect(result).toContain('hljs-keyword')
    expect(result).not.toContain('<script')
  })
})
