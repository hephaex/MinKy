"""Application-wide constants.

Centralizes magic numbers and configuration values for maintainability.
"""

# Pagination defaults
DEFAULT_PAGE_SIZE = 20
MAX_PAGE_SIZE = 100

# Query limits
MAX_ANALYTICS_DAYS = 365
MAX_QUERY_HOURS = 168  # 7 days
MAX_SEARCH_RESULTS = 100
MAX_SIMILAR_DOCUMENTS = 100
MAX_RATING_STATS_LIMIT = 10000
MAX_COMMENT_REPLIES = 100

# File size limits (in bytes)
MAX_BACKUP_SIZE_MB = 10
MAX_UPLOAD_SIZE_MB = 50

# Text limits
MAX_COMMIT_MESSAGE_LENGTH = 500
MAX_TITLE_LENGTH = 255
MAX_TAG_NAME_LENGTH = 50

# Cache TTL (in seconds)
DEFAULT_CACHE_TTL = 300  # 5 minutes
STATS_CACHE_TTL = 600    # 10 minutes
