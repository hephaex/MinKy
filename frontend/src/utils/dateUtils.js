/**
 * Centralized date formatting utilities
 * Replaces duplicated formatDate functions across components
 */

/**
 * Format a date string to a localized date format
 * @param {string|Date} dateString - Date to format
 * @param {string} locale - Locale string (default: 'en-US')
 * @returns {string} Formatted date string
 */
export const formatDate = (dateString, locale = 'en-US') => {
  if (!dateString) return '';

  try {
    return new Date(dateString).toLocaleDateString(locale, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  } catch {
    return '';
  }
};

/**
 * Format a date string to include time
 * @param {string|Date} dateString - Date to format
 * @param {string} locale - Locale string (default: 'en-US')
 * @returns {string} Formatted date and time string
 */
export const formatDateTime = (dateString, locale = 'en-US') => {
  if (!dateString) return '';

  try {
    return new Date(dateString).toLocaleDateString(locale, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  } catch {
    return '';
  }
};

/**
 * Format a date range for display (e.g., "2024", "2024-01", "2024-01-15")
 * @param {string} dateRange - Date range string in YYYY, YYYY-MM, or YYYY-MM-DD format
 * @param {string} locale - Locale string (default: 'en-US')
 * @returns {string} Formatted date range string
 */
export const formatDateRange = (dateRange, locale = 'en-US') => {
  if (!dateRange) return '';

  const parts = dateRange.split('-');

  try {
    if (parts.length === 1) {
      // Year only: "2024"
      return parts[0];
    } else if (parts.length === 2) {
      // Year and month: "2024-01"
      const date = new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, 1);
      return date.toLocaleDateString(locale, { year: 'numeric', month: 'long' });
    } else if (parts.length === 3) {
      // Full date: "2024-01-15"
      const date = new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, parseInt(parts[2]));
      return date.toLocaleDateString(locale, { year: 'numeric', month: 'long', day: 'numeric' });
    }
  } catch {
    return dateRange;
  }

  return dateRange;
};

/**
 * Format relative time (e.g., "2 hours ago", "yesterday")
 * @param {string|Date} dateString - Date to format
 * @param {string} locale - Locale string (default: 'en-US')
 * @returns {string} Relative time string
 */
export const formatRelativeTime = (dateString, locale = 'en-US') => {
  if (!dateString) return '';

  try {
    const date = new Date(dateString);
    const now = new Date();
    const diffInSeconds = Math.floor((now - date) / 1000);

    const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });

    if (diffInSeconds < 60) {
      return rtf.format(-diffInSeconds, 'second');
    }

    const diffInMinutes = Math.floor(diffInSeconds / 60);
    if (diffInMinutes < 60) {
      return rtf.format(-diffInMinutes, 'minute');
    }

    const diffInHours = Math.floor(diffInMinutes / 60);
    if (diffInHours < 24) {
      return rtf.format(-diffInHours, 'hour');
    }

    const diffInDays = Math.floor(diffInHours / 24);
    if (diffInDays < 30) {
      return rtf.format(-diffInDays, 'day');
    }

    const diffInMonths = Math.floor(diffInDays / 30);
    if (diffInMonths < 12) {
      return rtf.format(-diffInMonths, 'month');
    }

    const diffInYears = Math.floor(diffInMonths / 12);
    return rtf.format(-diffInYears, 'year');
  } catch {
    return formatDate(dateString, locale);
  }
};
