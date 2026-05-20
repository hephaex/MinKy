"""
Unit tests for utility modules: obsidian_parser and auto_tag.
These are pure unit tests without Flask app context.
"""
import pytest
from app.utils.obsidian_parser import ObsidianParser
from app.utils.auto_tag import detect_auto_tags, merge_tags, generate_tags_from_content


class TestObsidianParser:
    """Test suite for ObsidianParser class."""

    def test_obsidian_parse_frontmatter(self):
        """Test parsing markdown with YAML frontmatter."""
        parser = ObsidianParser()
        content = """---
title: Test Document
author: John Doe
tags:
  - python
  - testing
date: 2024-01-15
---

# Main Content

This is the actual content."""

        result = parser.parse_markdown(content)

        # Verify frontmatter extraction
        assert 'frontmatter' in result
        assert result['frontmatter']['title'] == 'Test Document'
        assert result['frontmatter']['author'] == 'John Doe'
        assert result['frontmatter']['tags'] == ['python', 'testing']
        assert result['frontmatter']['date'] == '2024-01-15'

        # Verify frontmatter is removed from clean content
        assert 'clean_content' in result
        assert '# Main Content' in result['clean_content']
        assert 'This is the actual content.' in result['clean_content']

    def test_obsidian_parse_frontmatter_empty(self):
        """Test parsing markdown without frontmatter."""
        parser = ObsidianParser()
        content = "# Just Content\n\nNo frontmatter here."

        result = parser.parse_markdown(content)

        assert result['frontmatter'] == {}
        assert result['clean_content'] == content

    def test_obsidian_extract_internal_links(self):
        """Test parsing [[link]] and [[link|display]] patterns."""
        parser = ObsidianParser()
        content = """
# Test Document

This links to [[Another Document]].
Here's a link with custom text: [[Target|Custom Display]].
Multiple [[Link1]] and [[Link2|Display2]] in one line.
"""

        result = parser.parse_markdown(content)

        # Verify internal links extraction
        assert 'internal_links' in result
        links = result['internal_links']
        assert len(links) == 4

        # First link: [[Another Document]]
        assert links[0]['target'] == 'Another Document'
        assert links[0]['display_text'] == 'Another Document'
        assert links[0]['raw_match'] == '[[Another Document]]'

        # Second link: [[Target|Custom Display]]
        assert links[1]['target'] == 'Target'
        assert links[1]['display_text'] == 'Custom Display'
        assert links[1]['raw_match'] == '[[Target|Custom Display]]'

        # Third link: [[Link1]]
        assert links[2]['target'] == 'Link1'
        assert links[2]['display_text'] == 'Link1'

        # Fourth link: [[Link2|Display2]]
        assert links[3]['target'] == 'Link2'
        assert links[3]['display_text'] == 'Display2'

    def test_obsidian_render_internal_links(self):
        """Test rendering internal links to HTML."""
        parser = ObsidianParser()
        content = "Link to [[Document]] and [[Other|Custom]]."

        # Test without lookup function
        rendered = parser.render_internal_links(content)
        assert '<span class="internal-link-placeholder" data-target="Document">Document</span>' in rendered
        assert '<span class="internal-link-placeholder" data-target="Other">Custom</span>' in rendered

        # Test with lookup function
        def lookup_func(title):
            if title == 'Document':
                return 123
            return None

        rendered_with_lookup = parser.render_internal_links(content, lookup_func)
        assert '<a href="/documents/123" class="internal-link">Document</a>' in rendered_with_lookup
        assert '<a href="#" class="internal-link broken" data-target="Other">Custom</a>' in rendered_with_lookup

    def test_obsidian_extract_hashtags(self):
        """Test parsing #tag patterns."""
        parser = ObsidianParser()
        content = """
# Test Document

This has #python and #testing tags.
Korean tag: #파이썬
Mixed: #python-3 and #test_case
At start #beginning
#duplicate and #duplicate should appear once
"""

        result = parser.parse_markdown(content)

        # Verify hashtags extraction
        assert 'hashtags' in result
        hashtags = result['hashtags']

        # Extract just the tag names for easier testing
        tag_names = [tag['tag'] for tag in hashtags]

        assert 'python' in tag_names
        assert 'testing' in tag_names
        assert '파이썬' in tag_names
        assert 'python-3' in tag_names
        assert 'test_case' in tag_names
        assert 'beginning' in tag_names

        # Verify duplicates are removed
        assert tag_names.count('duplicate') == 1

    def test_obsidian_render_hashtags(self):
        """Test rendering hashtags to HTML links."""
        parser = ObsidianParser()
        content = "This has #python and #testing tags."

        rendered = parser.render_hashtags(content)

        assert '<a href="/tags/python" class="hashtag">#python</a>' in rendered
        assert '<a href="/tags/testing" class="hashtag">#testing</a>' in rendered


class TestAutoTag:
    """Test suite for auto_tag module functions."""

    def test_auto_tag_detect_python(self):
        """Test content with Python keywords is detected."""
        content = """
import pandas as pd
import numpy as np

def calculate():
    df = pd.DataFrame()
    return df
"""

        tags = detect_auto_tags(content)

        assert 'Python' in tags
        assert 'pandas' in tags
        assert 'numpy' in tags

    def test_auto_tag_detect_python_libraries(self):
        """Test detection of various Python libraries."""
        test_cases = [
            ('import yfinance as yf', ['yfinance', 'Python']),
            ('import matplotlib.pyplot as plt', ['matplotlib', 'Python']),
            ('import mplfinance as mpf', ['mplfinance', 'Python', 'matplotlib']),
            ('from plotly import graph_objects', ['plotly', 'Python']),
            ('import seaborn as sns', ['seaborn', 'Python', 'matplotlib']),
        ]

        for content, expected_tags in test_cases:
            tags = detect_auto_tags(content)
            for expected_tag in expected_tags:
                assert expected_tag in tags, f"Expected {expected_tag} in tags for content: {content}"

    def test_auto_tag_detect_hashtags(self):
        """Test content with #hashtag is detected."""
        content = """
# My Document

This is about #python and #dataanalysis.
Also covers #machinelearning.
"""

        tags = detect_auto_tags(content)

        assert 'python' in tags
        assert 'dataanalysis' in tags
        assert 'machinelearning' in tags

    def test_auto_tag_detect_code_blocks(self):
        """Test detection of Python code blocks."""
        content = """
Here's some code:

```python
def hello():
    print("Hello, World!")
```
"""

        tags = detect_auto_tags(content)

        assert 'Python' in tags

    def test_auto_tag_detect_keywords(self):
        """Test detection of keywords in prose."""
        content = """
This tutorial covers pandas DataFrames and numpy arrays.
We'll use matplotlib for visualization.
"""

        tags = detect_auto_tags(content)

        assert 'Python' in tags
        assert 'pandas' in tags
        assert 'numpy' in tags
        assert 'matplotlib' in tags

    def test_auto_tag_empty_content(self):
        """Test detection with empty content."""
        tags = detect_auto_tags('')
        assert tags == []

        tags = detect_auto_tags(None)
        assert tags == []

    def test_merge_tags_no_duplicates(self):
        """Test merge with case-insensitive deduplication."""
        existing = ['Python', 'Testing', 'API']
        auto = ['python', 'numpy', 'TESTING', 'Docker']

        merged = merge_tags(existing, auto)

        # Convert to lowercase for comparison
        merged_lower = [tag.lower() for tag in merged]

        # No duplicates
        assert len(merged_lower) == len(set(merged_lower))

        # All unique tags present
        assert 'python' in merged_lower
        assert 'testing' in merged_lower
        assert 'api' in merged_lower
        assert 'numpy' in merged_lower
        assert 'docker' in merged_lower

        # Original case should be preserved (existing tags take precedence)
        assert 'Python' in merged
        assert 'Testing' in merged

    def test_merge_tags_empty_lists(self):
        """Test merge with empty lists."""
        assert merge_tags([], []) == []
        assert merge_tags(['Python'], []) == ['Python']
        assert merge_tags([], ['Python']) == ['Python']

    def test_merge_tags_preserves_order(self):
        """Test that existing tags maintain their order."""
        existing = ['A', 'B', 'C']
        auto = ['D', 'E']

        merged = merge_tags(existing, auto)

        # Existing tags should appear first
        assert merged[:3] == ['A', 'B', 'C']

    def test_generate_tags_from_content(self):
        """Test generate_tags_from_content with title analysis."""
        content = """
import pandas as pd

This document covers data analysis with pandas.
"""
        title = "Python Data Analysis Tutorial"

        tags = generate_tags_from_content(content, title)

        # Tags from content
        assert 'pandas' in tags
        assert 'Python' in tags

    def test_generate_tags_from_content_no_title(self):
        """Test generate_tags_from_content without title."""
        content = "Using yfinance for stock data #finance #stocks"

        tags = generate_tags_from_content(content)

        assert 'yfinance' in tags
        assert 'Python' in tags
        assert 'finance' in tags
        assert 'stocks' in tags

    def test_auto_tag_filters_clippings(self):
        """Test that 'clippings' tag is filtered out."""
        content = "This is about #clippings and #python"

        tags = detect_auto_tags(content)

        assert 'clippings' not in [tag.lower() for tag in tags]
        assert 'python' in tags


class TestObsidianParserEdgeCases:
    """Test edge cases and error handling for ObsidianParser."""

    def test_invalid_yaml_frontmatter(self):
        """Test handling of malformed YAML frontmatter."""
        parser = ObsidianParser()
        content = """---
invalid: yaml: structure:
  - unclosed
---

Content"""

        result = parser.parse_markdown(content)

        # Should handle gracefully
        assert result['frontmatter'] == {}
        assert 'Content' in result['clean_content']

    def test_nested_internal_links(self):
        """Test that nested brackets don't break parsing."""
        parser = ObsidianParser()
        content = "Link [[Document [with brackets]]] test"

        # Should extract the link without issues
        result = parser.parse_markdown(content)
        links = result['internal_links']

        # The regex will capture up to first ] or |
        assert len(links) > 0

    def test_hashtag_word_boundaries(self):
        """Test that hashtags respect word boundaries."""
        parser = ObsidianParser()
        content = "email@test.com should not be #test tag. But #test is a tag."

        result = parser.parse_markdown(content)
        hashtags = result['hashtags']

        tag_names = [tag['tag'] for tag in hashtags]
        assert 'test' in tag_names

    def test_unicode_in_links_and_tags(self):
        """Test Unicode characters in links and tags."""
        parser = ObsidianParser()
        content = "[[한글 문서]] and #한글태그"

        result = parser.parse_markdown(content)

        assert len(result['internal_links']) == 1
        assert result['internal_links'][0]['target'] == '한글 문서'

        assert len(result['hashtags']) == 1
        assert result['hashtags'][0]['tag'] == '한글태그'


class TestAutoTagEdgeCases:
    """Test edge cases for auto_tag module."""

    def test_case_insensitive_keyword_matching(self):
        """Test that keyword matching is case-insensitive."""
        content = "Using PANDAS and NumPy for data analysis with MatPlotLib"

        tags = detect_auto_tags(content)

        assert 'pandas' in tags
        assert 'numpy' in tags
        assert 'matplotlib' in tags

    def test_word_boundary_matching(self):
        """Test that keywords must be whole words."""
        content = "mypandas is not pandas, but pandas is"

        tags = detect_auto_tags(content)

        # Should only detect the second occurrence
        assert 'pandas' in tags

    def test_multiple_import_styles(self):
        """Test various import statement styles."""
        test_cases = [
            "import yfinance",
            "import yfinance as yf",
            "from yfinance import Ticker",
            "from yfinance.ticker import Ticker",
        ]

        for content in test_cases:
            tags = detect_auto_tags(content)
            assert 'yfinance' in tags, f"Failed for: {content}"
            assert 'Python' in tags, f"Failed for: {content}"

    def test_merge_preserves_existing_case(self):
        """Test that merge preserves case of existing tags over auto tags."""
        existing = ['PyThOn', 'TeStiNg']
        auto = ['python', 'testing', 'numpy']

        merged = merge_tags(existing, auto)

        # Original case from existing should be preserved
        assert 'PyThOn' in merged
        assert 'TeStiNg' in merged
        # New tag uses case from auto
        assert 'numpy' in merged

        # No duplicates
        merged_lower = [tag.lower() for tag in merged]
        assert len(merged_lower) == len(set(merged_lower))
