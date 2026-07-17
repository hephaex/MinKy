/**
 * Reduce a markdown fragment to plain text for card previews.
 * Handles fragments cut mid-syntax (e.g. an unterminated code fence in a
 * server-side excerpt).
 */
export const stripMarkdown = (text) => {
  if (!text) return '';
  return (
    text
      // fenced code blocks, including a trailing unterminated fence
      .replace(/```[\s\S]*?(```|$)/g, ' ')
      .replace(/`([^`]*)`/g, '$1')
      .replace(/!\[([^\]]*)\]\([^)]*\)/g, '$1')
      .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
      .replace(/^#{1,6}\s+/gm, '')
      .replace(/^\s*[-*+]\s+/gm, '')
      .replace(/^\s*\d+\.\s+/gm, '')
      .replace(/(\*\*|__|~~)/g, '')
      .replace(/^\s*>\s?/gm, '')
      .replace(/\s+/g, ' ')
      .trim()
  );
};

export const formatAuthor = (rawAuthor) => {
  if (!rawAuthor) return '';
  if (typeof rawAuthor !== 'string' && !Array.isArray(rawAuthor)) return '';

  let author = rawAuthor;

  if (typeof author === 'string') {
    try {
      const parsed = JSON.parse(author);
      if (Array.isArray(parsed) && parsed.length > 0) {
        author = parsed[0];
      } else if (typeof parsed === 'string') {
        author = parsed;
      }
    } catch {
      // Not JSON, use as-is
    }
  }

  if (Array.isArray(author)) {
    if (author.length === 0) return '';
    author = author[0];
  }

  if (typeof author === 'string') {
    author = author.trim();
    if (author.startsWith('[[') && author.endsWith(']]')) {
      author = author.slice(2, -2);
    }
    author = author.replace(/^["']|["']$/g, '');
  }

  return author;
};
