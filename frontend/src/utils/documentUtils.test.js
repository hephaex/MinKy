import { formatAuthor } from './documentUtils';

describe('formatAuthor', () => {
  test('returns empty string for null', () => {
    expect(formatAuthor(null)).toBe('');
  });

  test('returns empty string for undefined', () => {
    expect(formatAuthor(undefined)).toBe('');
  });

  test('returns empty string for empty string', () => {
    expect(formatAuthor('')).toBe('');
  });

  test('returns plain string as-is', () => {
    expect(formatAuthor('John Doe')).toBe('John Doe');
  });

  test('trims whitespace', () => {
    expect(formatAuthor('  Jane Doe  ')).toBe('Jane Doe');
  });

  test('unwraps JSON array string', () => {
    expect(formatAuthor('["Alice"]')).toBe('Alice');
  });

  test('takes first element from JSON array string', () => {
    expect(formatAuthor('["Alice", "Bob"]')).toBe('Alice');
  });

  test('unwraps JSON string', () => {
    expect(formatAuthor('"Charlie"')).toBe('Charlie');
  });

  test('handles actual array input', () => {
    expect(formatAuthor(['Dave', 'Eve'])).toBe('Dave');
  });

  test('handles empty array', () => {
    expect(formatAuthor([])).toBe('');
  });

  test('strips Obsidian wiki links', () => {
    expect(formatAuthor('[[Frank]]')).toBe('Frank');
  });

  test('strips surrounding quotes', () => {
    expect(formatAuthor("'Grace'")).toBe('Grace');
    expect(formatAuthor('"Heidi"')).toBe('Heidi');
  });

  test('handles wiki links inside JSON array', () => {
    expect(formatAuthor('["[[Ivan]]"]')).toBe('Ivan');
  });

  test('does not mutate original string input', () => {
    const original = '  [[Kim]]  ';
    formatAuthor(original);
    expect(original).toBe('  [[Kim]]  ');
  });

  test('returns non-string non-array as empty', () => {
    expect(formatAuthor(42)).toBe(42);
  });

  test('handles JSON parse failure gracefully', () => {
    expect(formatAuthor('{invalid json')).toBe('{invalid json');
  });
});
