import { slugify, extractHeadings } from './tocUtils';

describe('slugify', () => {
  it('lowercases and hyphenates plain ascii text', () => {
    expect(slugify('Installation Guide')).toBe('installation-guide');
  });

  it('keeps Korean characters readable', () => {
    expect(slugify('설치 방법')).toBe('설치-방법');
  });

  it('strips emphasis/code markers and punctuation', () => {
    expect(slugify('**Bold** `code` and (punct)!')).toBe('bold-code-and-punct');
  });

  it('collapses repeated whitespace/hyphens into one', () => {
    expect(slugify('a   b---c')).toBe('a-b-c');
  });

  it('falls back to "section" for text with no sluggable characters', () => {
    expect(slugify('***')).toBe('section');
    expect(slugify('')).toBe('section');
  });
});

describe('extractHeadings', () => {
  it('returns [] for empty/undefined input', () => {
    expect(extractHeadings('')).toEqual([]);
    expect(extractHeadings(undefined)).toEqual([]);
  });

  it('extracts ATX headings in order with level and slug', () => {
    const md = ['# Title', 'some text', '## Section One', 'more text', '### Sub'].join('\n');
    expect(extractHeadings(md)).toEqual([
      { level: 1, text: 'Title', slug: 'title' },
      { level: 2, text: 'Section One', slug: 'section-one' },
      { level: 3, text: 'Sub', slug: 'sub' },
    ]);
  });

  it('ignores "#" characters inside fenced code blocks', () => {
    const md = ['# Real Heading', '```', '# not a heading', '```', '## Also Real'].join('\n');
    expect(extractHeadings(md)).toEqual([
      { level: 1, text: 'Real Heading', slug: 'real-heading' },
      { level: 2, text: 'Also Real', slug: 'also-real' },
    ]);
  });

  it('ignores fenced blocks using the ~~~ marker', () => {
    const md = ['~~~', '# not a heading', '~~~', '# Real'].join('\n');
    expect(extractHeadings(md)).toEqual([{ level: 1, text: 'Real', slug: 'real' }]);
  });

  it('de-duplicates identical heading text with -1, -2 suffixes', () => {
    const md = ['## Overview', '## Overview', '## Overview'].join('\n');
    expect(extractHeadings(md)).toEqual([
      { level: 2, text: 'Overview', slug: 'overview' },
      { level: 2, text: 'Overview', slug: 'overview-1' },
      { level: 2, text: 'Overview', slug: 'overview-2' },
    ]);
  });

  it('strips optional trailing "#" closing sequences', () => {
    expect(extractHeadings('## Title ##')).toEqual([{ level: 2, text: 'Title', slug: 'title' }]);
  });

  it('does not treat a bare "#" (no heading text) as a heading', () => {
    expect(extractHeadings('#\nnot a heading either')).toEqual([]);
  });

  it('does not match "#tag" (no space after hash) as a heading', () => {
    expect(extractHeadings('#hashtag not a heading')).toEqual([]);
  });
});
