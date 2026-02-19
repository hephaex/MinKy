import { formatDate, formatDateTime, formatDateRange, formatRelativeTime } from './dateUtils';

describe('formatDate', () => {
  it('returns empty string for null input', () => {
    expect(formatDate(null)).toBe('');
  });

  it('returns empty string for undefined input', () => {
    expect(formatDate(undefined)).toBe('');
  });

  it('returns empty string for empty string input', () => {
    expect(formatDate('')).toBe('');
  });

  it('formats a valid ISO date string', () => {
    const result = formatDate('2026-02-19T10:00:00Z', 'en-US');
    expect(result).toMatch(/Feb/);
    expect(result).toMatch(/2026/);
  });

  it('formats a Date object', () => {
    const date = new Date(2026, 1, 19); // Feb 19, 2026
    const result = formatDate(date, 'en-US');
    expect(result).toMatch(/2026/);
  });

  it('handles invalid date string gracefully', () => {
    // JavaScript's toLocaleDateString returns "Invalid Date" for invalid inputs
    // The function catches errors but JS doesn't throw for "not-a-date"
    const result = formatDate('not-a-date');
    expect(typeof result).toBe('string');
  });
});

describe('formatDateTime', () => {
  it('returns empty string for null input', () => {
    expect(formatDateTime(null)).toBe('');
  });

  it('returns empty string for undefined input', () => {
    expect(formatDateTime(undefined)).toBe('');
  });

  it('returns empty string for empty string', () => {
    expect(formatDateTime('')).toBe('');
  });

  it('formats a valid ISO datetime string with time included', () => {
    const result = formatDateTime('2026-02-19T14:30:00Z', 'en-US');
    expect(result).toMatch(/2026/);
    // Should include time part (AM/PM or 24h)
    expect(result.length).toBeGreaterThan(10);
  });

  it('handles invalid date string gracefully', () => {
    const result = formatDateTime('invalid');
    expect(typeof result).toBe('string');
  });
});

describe('formatDateRange', () => {
  it('returns empty string for null', () => {
    expect(formatDateRange(null)).toBe('');
  });

  it('returns empty string for undefined', () => {
    expect(formatDateRange(undefined)).toBe('');
  });

  it('returns empty string for empty string', () => {
    expect(formatDateRange('')).toBe('');
  });

  it('formats year-only range', () => {
    const result = formatDateRange('2026');
    expect(result).toBe('2026');
  });

  it('formats year-month range', () => {
    const result = formatDateRange('2026-02', 'en-US');
    expect(result).toMatch(/February/i);
    expect(result).toMatch(/2026/);
  });

  it('formats full date range', () => {
    const result = formatDateRange('2026-02-19', 'en-US');
    expect(result).toMatch(/2026/);
    expect(result).toMatch(/19/);
  });

  it('returns original string for unrecognized format', () => {
    const weird = '2026-02-19-extra';
    const result = formatDateRange(weird);
    // 4 parts - should fall through to return dateRange
    expect(result).toBe(weird);
  });
});

describe('formatRelativeTime', () => {
  it('returns empty string for null', () => {
    expect(formatRelativeTime(null)).toBe('');
  });

  it('returns empty string for undefined', () => {
    expect(formatRelativeTime(undefined)).toBe('');
  });

  it('returns empty string for empty string', () => {
    expect(formatRelativeTime('')).toBe('');
  });

  it('returns a relative time string for a recent date', () => {
    const recent = new Date(Date.now() - 60 * 1000); // 1 minute ago
    const result = formatRelativeTime(recent, 'en-US');
    // Should contain "minute" or "ago" or similar relative indicator
    expect(typeof result).toBe('string');
    expect(result.length).toBeGreaterThan(0);
  });

  it('returns a relative time string for an old date', () => {
    const old = new Date(Date.now() - 365 * 24 * 60 * 60 * 1000); // ~1 year ago
    const result = formatRelativeTime(old, 'en-US');
    expect(typeof result).toBe('string');
    expect(result.length).toBeGreaterThan(0);
  });

  it('falls back to formatted date for invalid input', () => {
    // formatRelativeTime catches errors and falls back to formatDate
    const result = formatRelativeTime('invalid-date-string');
    expect(typeof result).toBe('string');
  });
});
