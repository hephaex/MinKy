export const formatAuthor = (rawAuthor) => {
  if (!rawAuthor) return '';

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

  if (Array.isArray(author) && author.length > 0) {
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
