import { highlightText, truncateWithHighlight } from './highlightText';

describe('highlightText', () => {
  it('returns original text when searchQuery is empty', () => {
    const text = 'Hello World';
    expect(highlightText(text, '')).toBe(text);
    expect(highlightText(text, null)).toBe(text);
    expect(highlightText(text, undefined)).toBe(text);
  });

  it('returns original text when text is empty', () => {
    expect(highlightText('', 'search')).toBe('');
    expect(highlightText(null, 'search')).toBe(null);
  });

  it('wraps matching text in mark tags', () => {
    const result = highlightText('Hello World', 'World');
    expect(result).toContain('<mark class="search-highlight">World</mark>');
  });

  it('is case insensitive', () => {
    const result = highlightText('Hello WORLD', 'world');
    expect(result).toContain('<mark class="search-highlight">WORLD</mark>');
  });

  it('highlights multiple occurrences', () => {
    const result = highlightText('test test test', 'test');
    const matches = result.match(/<mark/g);
    expect(matches).toHaveLength(3);
  });

  it('escapes special regex characters in query', () => {
    const result = highlightText('Price: $100', '$100');
    expect(result).toContain('<mark class="search-highlight">$100</mark>');
  });

  it('handles whitespace in query', () => {
    const result = highlightText('Hello World', '   ');
    expect(result).toBe('Hello World');
  });
});

describe('truncateWithHighlight', () => {
  it('truncates text to maxLength when no query', () => {
    const longText = 'A'.repeat(200);
    const result = truncateWithHighlight(longText, '', 150);
    expect(result).toHaveLength(153); // 150 + '...'
    expect(result.endsWith('...')).toBe(true);
  });

  it('returns full text if shorter than maxLength', () => {
    const shortText = 'Hello';
    const result = truncateWithHighlight(shortText, '', 150);
    expect(result).toBe('Hello');
  });

  it('centers excerpt around search query', () => {
    const text = 'A'.repeat(100) + 'KEYWORD' + 'B'.repeat(100);
    const result = truncateWithHighlight(text, 'KEYWORD', 50);
    expect(result).toContain('KEYWORD');
  });

  it('handles query not found', () => {
    const text = 'Hello World';
    const result = truncateWithHighlight(text, 'notfound', 150);
    expect(result).toBe('Hello World');
  });

  it('adds ellipsis at start when excerpt starts mid-text', () => {
    const text = 'A'.repeat(100) + 'KEYWORD';
    const result = truncateWithHighlight(text, 'KEYWORD', 30);
    expect(result.startsWith('...')).toBe(true);
  });

  it('handles empty query with whitespace', () => {
    const text = 'Hello World';
    const result = truncateWithHighlight(text, '   ', 150);
    expect(result).toBe('Hello World');
  });

  it('is case insensitive when finding query position', () => {
    const text = 'A'.repeat(100) + 'KEYWORD' + 'B'.repeat(100);
    const result = truncateWithHighlight(text, 'keyword', 50);
    expect(result).toContain('KEYWORD');
  });
});
