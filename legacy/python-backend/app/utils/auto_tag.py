import re
import logging
from typing import List

logger = logging.getLogger(__name__)


def detect_auto_tags(content: str) -> List[str]:
    """
    Detect automatic tags based on content analysis.

    Args:
        content (str): The markdown content to analyze

    Returns:
        List[str]: List of detected tags
    """
    if not content:
        return []

    logger.debug("Analyzing content: %s...", content[:100])

    # Keywords to detect and their corresponding tags
    AUTO_TAG_KEYWORDS = {
        'yfinance': ['yfinance', 'Python'],
        'mplfinance': ['mplfinance', 'Python', 'matplotlib'],
        'matplotlib': ['matplotlib', 'Python'],
        'python': ['Python'],
        'pandas': ['pandas', 'Python'],
        'numpy': ['numpy', 'Python'],
        'seaborn': ['seaborn', 'Python', 'matplotlib'],
        'plotly': ['plotly', 'Python'],
        'jupyter': ['Jupyter', 'Python'],
        'notebook': ['Jupyter', 'Python']
    }

    detected_tags = set()
    content_lower = content.lower()

    # 1. Check for hashtags (#태그)
    hashtag_pattern = r'#([가-힣\w]+)'
    hashtag_matches = re.findall(hashtag_pattern, content)
    for tag in hashtag_matches:
        detected_tags.add(tag)
        logger.debug("Found hashtag: #%s", tag)

    # 2. Check for keywords in content
    for keyword, tags in AUTO_TAG_KEYWORDS.items():
        # Create pattern to match keyword as whole word
        pattern = r'\b' + re.escape(keyword) + r'\b'
        if re.search(pattern, content_lower):
            detected_tags.update(tags)
            logger.debug("Found keyword: %s -> %s", keyword, tags)

    # 3. Check for import statements (Python specific)
    import_patterns = [
        r'import\s+yfinance',
        r'import\s+mplfinance',
        r'import\s+matplotlib',
        r'from\s+yfinance',
        r'from\s+mplfinance',
        r'from\s+matplotlib',
        r'import\s+pandas',
        r'import\s+numpy',
        r'import\s+seaborn',
        r'import\s+plotly'
    ]

    for pattern in import_patterns:
        if re.search(pattern, content_lower):
            if 'yfinance' in pattern:
                detected_tags.update(['yfinance', 'Python'])
            elif 'mplfinance' in pattern:
                detected_tags.update(['mplfinance', 'Python', 'matplotlib'])
            elif 'matplotlib' in pattern:
                detected_tags.update(['matplotlib', 'Python'])
            elif 'pandas' in pattern:
                detected_tags.update(['pandas', 'Python'])
            elif 'numpy' in pattern:
                detected_tags.update(['numpy', 'Python'])
            elif 'seaborn' in pattern:
                detected_tags.update(['seaborn', 'Python', 'matplotlib'])
            elif 'plotly' in pattern:
                detected_tags.update(['plotly', 'Python'])

    # 4. Check for code blocks with python
    python_code_pattern = r'```python'
    if re.search(python_code_pattern, content_lower):
        detected_tags.add('Python')
        logger.debug("Found Python code block")

    # Filter out unwanted automatic tags
    filtered_tags = [tag for tag in detected_tags if tag and tag.lower() != 'clippings']

    result = list(filtered_tags)
    logger.debug("Final detected tags: %s", result)
    return result


def merge_tags(existing_tags: List[str], auto_tags: List[str]) -> List[str]:
    """
    Merge existing tags with automatically detected tags, avoiding duplicates.

    Args:
        existing_tags (List[str]): Existing tags
        auto_tags (List[str]): Automatically detected tags

    Returns:
        List[str]: Merged list of unique tags
    """
    if not existing_tags:
        existing_tags = []
    if not auto_tags:
        auto_tags = []

    logger.debug("Existing tags: %s", existing_tags)
    logger.debug("Auto tags: %s", auto_tags)

    # Convert to lowercase for comparison, but preserve original case
    existing_lower = {tag.lower(): tag for tag in existing_tags}
    merged_tags = dict(existing_lower)

    # Add auto tags if not already present (case-insensitive)
    for tag in auto_tags:
        tag_lower = tag.lower()
        if tag_lower not in merged_tags:
            merged_tags[tag_lower] = tag

    result = list(merged_tags.values())
    logger.debug("Final merged tags: %s", result)
    return result


def generate_tags_from_content(content: str, title: str = '') -> List[str]:
    """
    Generate tags from content and title.
    This is an alias for detect_auto_tags with additional title analysis.

    Args:
        content (str): The content to analyze
        title (str): The title to analyze (optional)

    Returns:
        List[str]: List of generated tags
    """
    # Start with auto-detected tags from content
    tags = detect_auto_tags(content)

    # Add title-based tags if title is provided
    if title:
        title_tags = detect_auto_tags(title)
        tags = merge_tags(tags, title_tags)

    return tags
