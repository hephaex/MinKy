import re
from typing import List, Set


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
    
    print(f"[AUTO_TAG] Analyzing content: {content[:100]}...")  # Debug log
    
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
        print(f"[AUTO_TAG] Found hashtag: #{tag}")
    
    # 2. Check for keywords in content
    for keyword, tags in AUTO_TAG_KEYWORDS.items():
        # Create pattern to match keyword as whole word
        pattern = r'\b' + re.escape(keyword) + r'\b'
        if re.search(pattern, content_lower):
            detected_tags.update(tags)
            print(f"[AUTO_TAG] Found keyword: {keyword} -> {tags}")
    
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
                print(f"[AUTO_TAG] Found import: {pattern} -> yfinance, Python")
            elif 'mplfinance' in pattern:
                detected_tags.update(['mplfinance', 'Python', 'matplotlib'])
                print(f"[AUTO_TAG] Found import: {pattern} -> mplfinance, Python, matplotlib")
            elif 'matplotlib' in pattern:
                detected_tags.update(['matplotlib', 'Python'])
                print(f"[AUTO_TAG] Found import: {pattern} -> matplotlib, Python")
            elif 'pandas' in pattern:
                detected_tags.update(['pandas', 'Python'])
                print(f"[AUTO_TAG] Found import: {pattern} -> pandas, Python")
            elif 'numpy' in pattern:
                detected_tags.update(['numpy', 'Python'])
                print(f"[AUTO_TAG] Found import: {pattern} -> numpy, Python")
            elif 'seaborn' in pattern:
                detected_tags.update(['seaborn', 'Python', 'matplotlib'])
                print(f"[AUTO_TAG] Found import: {pattern} -> seaborn, Python, matplotlib")
            elif 'plotly' in pattern:
                detected_tags.update(['plotly', 'Python'])
                print(f"[AUTO_TAG] Found import: {pattern} -> plotly, Python")
    
    # 4. Check for code blocks with python
    python_code_pattern = r'```python'
    if re.search(python_code_pattern, content_lower):
        detected_tags.add('Python')
        print(f"[AUTO_TAG] Found Python code block")
    
    # Filter out unwanted automatic tags
    filtered_tags = [tag for tag in detected_tags if tag and tag.lower() != 'clippings']
    
    result = list(filtered_tags)
    print(f"[AUTO_TAG] Final detected tags: {result}")
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
    
    print(f"[MERGE_TAGS] Existing tags: {existing_tags}")
    print(f"[MERGE_TAGS] Auto tags: {auto_tags}")
    
    # Convert to lowercase for comparison, but preserve original case
    existing_lower = {tag.lower(): tag for tag in existing_tags}
    merged_tags = dict(existing_lower)
    
    # Add auto tags if not already present (case-insensitive)
    for tag in auto_tags:
        tag_lower = tag.lower()
        if tag_lower not in merged_tags:
            merged_tags[tag_lower] = tag
    
    result = list(merged_tags.values())
    print(f"[MERGE_TAGS] Final merged tags: {result}")
    return result