// Mock heavy dependencies before importing from obsidianRenderer
jest.mock('react-syntax-highlighter', () => ({
  Prism: ({ children }) => children,
}));
jest.mock('react-syntax-highlighter/dist/esm/styles/prism', () => ({
  tomorrow: {},
}));
jest.mock('dompurify', () => ({
  sanitize: jest.fn((html) => html),
}));

import { processInternalLinks, processHashtags, extractFrontmatter } from './obsidianRenderer';

describe('processInternalLinks', () => {
  it('returns original content when no internal links', () => {
    const content = 'Hello World';
    expect(processInternalLinks(content, null)).toBe('Hello World');
  });

  it('replaces [[link]] with broken span when no lookup', () => {
    const content = '[[MyDoc]]';
    const result = processInternalLinks(content, null, {});
    expect(result).toContain('class="internal-link broken"');
    expect(result).toContain('MyDoc');
  });

  it('replaces [[link]] with anchor when doc found in lookup', () => {
    const content = '[[MyDoc]]';
    const result = processInternalLinks(content, null, { MyDoc: 'uuid-123' });
    expect(result).toContain('href="/documents/uuid-123"');
    expect(result).toContain('MyDoc');
  });

  it('uses display text from [[link|display]]', () => {
    const content = '[[InternalRef|Friendly Name]]';
    const result = processInternalLinks(content, null, { InternalRef: 'uuid-456' });
    expect(result).toContain('Friendly Name');
    expect(result).not.toContain('InternalRef<');
  });

  it('shows display text for broken link with alias', () => {
    const content = '[[Missing|Alias]]';
    const result = processInternalLinks(content, null, {});
    expect(result).toContain('Alias');
    expect(result).toContain('class="internal-link broken"');
  });

  it('escapes HTML special chars in target to prevent injection', () => {
    const content = '[[<script>alert(1)</script>]]';
    const result = processInternalLinks(content, null, {});
    expect(result).not.toContain('<script>');
    expect(result).toContain('&lt;script&gt;');
  });

  it('handles multiple internal links in content', () => {
    const content = '[[DocA]] and [[DocB]]';
    const result = processInternalLinks(content, null, { DocA: 'id-a' });
    expect(result).toContain('href="/documents/id-a"');
    expect(result).toContain('class="internal-link broken"');
  });

  it('returns unchanged content when no link pattern found', () => {
    const content = 'Just plain text without links.';
    expect(processInternalLinks(content, null, {})).toBe(content);
  });
});

describe('processHashtags', () => {
  it('returns original content when no hashtags', () => {
    const content = 'Hello World';
    expect(processHashtags(content)).toBe('Hello World');
  });

  it('converts #tag to an anchor link', () => {
    const content = 'Check #rust for details';
    const result = processHashtags(content);
    expect(result).toContain('href="/tags/rust"');
    expect(result).toContain('#rust');
  });

  it('handles Korean hashtags', () => {
    const content = '안녕 #한국어태그 테스트';
    const result = processHashtags(content);
    expect(result).toContain('href="/tags/한국어태그"');
  });

  it('handles hashtag at start of line', () => {
    const content = '#firsttag text';
    const result = processHashtags(content);
    expect(result).toContain('href="/tags/firsttag"');
  });

  it('does not convert # with no space before it', () => {
    // hashtag pattern requires word start boundary (space or start)
    const content = 'text#nospace';
    const result = processHashtags(content);
    expect(result).not.toContain('href="/tags/nospace"');
  });

  it('handles multiple hashtags', () => {
    const content = 'tagged with #rust and #python';
    const result = processHashtags(content);
    expect(result).toContain('href="/tags/rust"');
    expect(result).toContain('href="/tags/python"');
  });
});

describe('extractFrontmatter', () => {
  it('returns empty metadata and full content when no frontmatter', () => {
    const content = 'Just a document without frontmatter.';
    const result = extractFrontmatter(content);
    expect(result.metadata).toEqual({});
    expect(result.content).toBe(content);
  });

  it('extracts basic key-value pairs', () => {
    const content = '---\ntitle: My Doc\nauthor: Alice\n---\nBody content';
    const result = extractFrontmatter(content);
    expect(result.metadata.title).toBe('My Doc');
    expect(result.metadata.author).toBe('Alice');
    expect(result.content).toBe('Body content');
  });

  it('strips double quotes from values', () => {
    const content = '---\ntitle: "Quoted Title"\n---\n';
    const result = extractFrontmatter(content);
    expect(result.metadata.title).toBe('Quoted Title');
  });

  it('strips single quotes from values', () => {
    const content = "---\nauthor: 'Single Quotes'\n---\n";
    const result = extractFrontmatter(content);
    expect(result.metadata.author).toBe('Single Quotes');
  });

  it('parses array values', () => {
    const content = '---\ntags: [rust, async, web]\n---\n';
    const result = extractFrontmatter(content);
    expect(Array.isArray(result.metadata.tags)).toBe(true);
    expect(result.metadata.tags).toContain('rust');
    expect(result.metadata.tags).toContain('async');
  });

  it('separates frontmatter from body content', () => {
    const content = '---\ntitle: Test\n---\nActual body here';
    const result = extractFrontmatter(content);
    expect(result.content).toBe('Actual body here');
    expect(result.content).not.toContain('title:');
  });

  it('handles empty frontmatter block', () => {
    const content = '---\n\n---\nContent';
    const result = extractFrontmatter(content);
    expect(result.metadata).toEqual({});
    expect(result.content).toBe('Content');
  });

  it('returns content unchanged if frontmatter is incomplete (no closing ---)', () => {
    const content = '---\ntitle: Incomplete\nNo closing';
    const result = extractFrontmatter(content);
    expect(result.metadata).toEqual({});
    expect(result.content).toBe(content);
  });

  it('returns content without frontmatter block when valid', () => {
    const content = '---\nkey: value\n---\nDocument body';
    const result = extractFrontmatter(content);
    expect(result.content).not.toContain('---');
  });
});
