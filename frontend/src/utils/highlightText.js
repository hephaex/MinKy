export const highlightText = (text, searchQuery) => {
  if (!searchQuery || !text) return text;
  
  const query = searchQuery.trim();
  if (!query) return text;
  
  // Create a regex that matches the search query (case insensitive)
  const regex = new RegExp(`(${escapeRegExp(query)})`, 'gi');
  
  // Split the text and wrap matches in highlight spans
  const parts = text.split(regex);
  
  return parts.map((part, index) => {
    if (regex.test(part)) {
      return `<mark class="search-highlight">${part}</mark>`;
    }
    return part;
  }).join('');
};

export const highlightTextReact = (text, searchQuery) => {
  if (!searchQuery || !text) return text;
  
  const query = searchQuery.trim();
  if (!query) return text;
  
  // Create a regex that matches the search query (case insensitive)
  const regex = new RegExp(`(${escapeRegExp(query)})`, 'gi');
  
  // Split the text and return React elements
  const parts = text.split(regex);
  
  return parts.map((part, index) => {
    if (regex.test(part)) {
      return <mark key={index} className="search-highlight">{part}</mark>;
    }
    return part;
  });
};

// Helper function to escape special regex characters
const escapeRegExp = (string) => {
  return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
};

export const truncateWithHighlight = (text, searchQuery, maxLength = 150) => {
  if (!searchQuery || !text) {
    return text.substring(0, maxLength) + (text.length > maxLength ? '...' : '');
  }
  
  const query = searchQuery.trim().toLowerCase();
  if (!query) {
    return text.substring(0, maxLength) + (text.length > maxLength ? '...' : '');
  }
  
  const lowerText = text.toLowerCase();
  const queryIndex = lowerText.indexOf(query);
  
  if (queryIndex === -1) {
    // Query not found, return normal truncation
    return text.substring(0, maxLength) + (text.length > maxLength ? '...' : '');
  }
  
  // Calculate start position to center the query
  const halfLength = Math.floor(maxLength / 2);
  let start = Math.max(0, queryIndex - halfLength);
  let end = Math.min(text.length, start + maxLength);
  
  // Adjust start if we're at the end
  if (end - start < maxLength) {
    start = Math.max(0, end - maxLength);
  }
  
  let excerpt = text.substring(start, end);
  
  // Add ellipsis
  if (start > 0) excerpt = '...' + excerpt;
  if (end < text.length) excerpt = excerpt + '...';
  
  return excerpt;
};