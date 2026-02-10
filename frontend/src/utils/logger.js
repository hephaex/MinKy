/**
 * Centralized logging utility for the frontend application.
 * In production, these could be sent to a logging service.
 * In development, they are output to the console.
 */

const isDevelopment = process.env.NODE_ENV === 'development';

/**
 * Log an error with context
 * @param {string} context - Where the error occurred
 * @param {Error|string} error - The error object or message
 * @param {Object} [metadata] - Additional context data
 */
export const logError = (context, error, metadata = {}) => {
  if (isDevelopment) {
    console.error(`[${context}]`, error, metadata);
  }
  // In production, could send to error tracking service (Sentry, etc.)
};

/**
 * Log a warning
 * @param {string} context - Where the warning occurred
 * @param {string} message - Warning message
 */
export const logWarning = (context, message) => {
  if (isDevelopment) {
    console.warn(`[${context}]`, message);
  }
};

/**
 * Log info for debugging
 * @param {string} context - Where the log occurred
 * @param {string} message - Info message
 */
export const logInfo = (context, message) => {
  if (isDevelopment) {
    console.info(`[${context}]`, message);
  }
};

export default { logError, logWarning, logInfo };
