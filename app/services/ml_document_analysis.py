"""
ML Document Analysis
Provides content analysis functions for individual documents
"""

import re
import logging
from typing import Dict, List, Any
from collections import Counter

logger = logging.getLogger(__name__)


def get_basic_document_stats(content: str) -> Dict[str, Any]:
    """Get basic statistical information about document content"""
    if not content:
        content = ''

    word_count = len(content.split())
    char_count = len(content)
    line_count = len(content.split('\n'))
    paragraph_count = len([p for p in content.split('\n\n') if p.strip()])

    header_count = len(re.findall(r'^#+\s', content, re.MULTILINE))
    link_count = len(re.findall(r'\[.*?\]\(.*?\)', content))
    image_count = len(re.findall(r'!\[.*?\]\(.*?\)', content))
    code_block_count = len(re.findall(r'```[\s\S]*?```', content))

    reading_time_minutes = max(1, word_count // 200)

    return {
        'word_count': word_count,
        'char_count': char_count,
        'line_count': line_count,
        'paragraph_count': paragraph_count,
        'header_count': header_count,
        'link_count': link_count,
        'image_count': image_count,
        'code_block_count': code_block_count,
        'reading_time_minutes': reading_time_minutes,
        'avg_words_per_paragraph': word_count / max(1, paragraph_count)
    }


def calculate_complexity_score(content: str) -> float:
    """Calculate a simple complexity score based on various factors"""
    if not content:
        return 0.0

    words = content.split()
    sentences = re.split(r'[.!?]+', content)

    if not words or not sentences:
        return 0.0

    avg_words_per_sentence = len(words) / len(sentences)
    avg_chars_per_word = sum(len(word) for word in words) / len(words)
    unique_words_ratio = len(set(words)) / len(words)
    complex_punct = len(re.findall(r'[;:(){}[\]"]', content)) / len(content)

    complexity = (
        (avg_words_per_sentence / 20.0) * 25 +
        (avg_chars_per_word / 8.0) * 25 +
        unique_words_ratio * 25 +
        (complex_punct * 1000) * 25
    )

    return min(100.0, complexity)


def calculate_keyword_density(content: str) -> Dict[str, float]:
    """Calculate keyword density for top terms"""
    words = re.findall(r'\b\w+\b', content.lower())

    if not words:
        return {}

    stop_words = {
        'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for',
        'of', 'with', 'by', 'is', 'are', 'was', 'were', 'be', 'been', 'have',
        'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could', 'should'
    }
    filtered_words = [word for word in words if len(word) > 3 and word not in stop_words]

    word_counts = Counter(filtered_words)
    total_words = len(filtered_words)

    keyword_density = {}
    for word, count in word_counts.most_common(10):
        keyword_density[word] = (count / total_words) * 100

    return keyword_density


def analyze_document_structure(content: str) -> Dict[str, Any]:
    """Analyze the structural elements of the document"""
    headers = re.findall(r'^(#+)\s+(.+)$', content, re.MULTILINE)

    structure: Dict[str, Any] = {
        'header_hierarchy': {},
        'section_lengths': [],
        'toc_depth': 0,
        'has_introduction': False,
        'has_conclusion': False
    }

    for level_marks, title in headers:
        level = len(level_marks)
        if level not in structure['header_hierarchy']:
            structure['header_hierarchy'][level] = []
        structure['header_hierarchy'][level].append(title.strip())
        structure['toc_depth'] = max(structure['toc_depth'], level)

    content_lower = content.lower()
    intro_patterns = ['introduction', 'overview', 'getting started', 'background']
    conclusion_patterns = ['conclusion', 'summary', 'final thoughts', 'wrap up']

    structure['has_introduction'] = any(pattern in content_lower for pattern in intro_patterns)
    structure['has_conclusion'] = any(pattern in content_lower for pattern in conclusion_patterns)

    sections = re.split(r'^#+\s+.+$', content, flags=re.MULTILINE)
    structure['section_lengths'] = [len(section.split()) for section in sections if section.strip()]

    return structure


def analyze_language_patterns(content: str) -> Dict[str, Any]:
    """Analyze language patterns and writing style"""
    patterns = {
        'question_count': len(re.findall(r'\?', content)),
        'exclamation_count': len(re.findall(r'!', content)),
        'code_ratio': 0.0,
        'list_items': len(re.findall(r'^\s*[-*+]\s', content, re.MULTILINE)),
        'numbered_items': len(re.findall(r'^\s*\d+\.\s', content, re.MULTILINE)),
        'emphasis_usage': {
            'bold': len(re.findall(r'\*\*.*?\*\*', content)),
            'italic': len(re.findall(r'\*.*?\*', content)),
            'code_inline': len(re.findall(r'`.*?`', content))
        }
    }

    code_blocks = re.findall(r'```[\s\S]*?```', content)
    code_chars = sum(len(block) for block in code_blocks)
    patterns['code_ratio'] = (code_chars / len(content)) * 100 if content else 0

    return patterns


def nltk_content_analysis(content: str, nltk_available: bool) -> Dict[str, Any]:
    """Perform NLTK-based content analysis"""
    if not nltk_available:
        return {}

    try:
        from nltk.tokenize import sent_tokenize, word_tokenize
        from nltk.tag import pos_tag

        sentences = sent_tokenize(content)
        words = word_tokenize(content.lower())

        pos_tags = pos_tag(words)
        pos_counts = Counter([tag for word, tag in pos_tags])

        total_tags = len(pos_tags)
        pos_distribution = {tag: (count / total_tags) * 100
                          for tag, count in pos_counts.most_common(5)}

        return {
            'sentence_count': len(sentences),
            'avg_sentence_length': len(words) / len(sentences) if sentences else 0,
            'pos_distribution': pos_distribution,
            'lexical_diversity': len(set(words)) / len(words) if words else 0
        }

    except Exception as e:
        logger.error(f"NLTK analysis error: {e}")
        return {}


def simple_sentiment_analysis(content: str) -> Dict[str, Any]:
    """Simple rule-based sentiment analysis as fallback"""
    positive_words = ['good', 'great', 'excellent', 'amazing', 'wonderful',
                      'fantastic', 'love', 'like', 'best', 'awesome']
    negative_words = ['bad', 'terrible', 'awful', 'horrible', 'hate',
                      'dislike', 'worst', 'poor', 'disappointing']

    content_lower = content.lower()
    positive_count = sum(1 for word in positive_words if word in content_lower)
    negative_count = sum(1 for word in negative_words if word in content_lower)

    if positive_count > negative_count:
        sentiment = 'positive'
        polarity = min(1.0, (positive_count - negative_count) / 10)
    elif negative_count > positive_count:
        sentiment = 'negative'
        polarity = max(-1.0, (positive_count - negative_count) / 10)
    else:
        sentiment = 'neutral'
        polarity = 0.0

    return {
        'sentiment': sentiment,
        'polarity': polarity,
        'subjectivity': 0.5,
        'confidence': min(1.0, abs(polarity))
    }


def get_document_recommendations(word_count: int, header_count: int,
                                  reading_time_minutes: int, link_count: int) -> List[Dict[str, Any]]:
    """Generate recommendations for improving the document"""
    recommendations = []

    if word_count < 100:
        recommendations.append({
            'type': 'content_length',
            'severity': 'high',
            'message': 'Document is very short. Consider adding more detailed content.',
            'suggestion': 'Aim for at least 200-300 words for better engagement.'
        })
    elif word_count > 3000:
        recommendations.append({
            'type': 'content_length',
            'severity': 'medium',
            'message': 'Document is quite long. Consider breaking it into sections.',
            'suggestion': 'Use more headers to organize content into digestible sections.'
        })

    if header_count == 0 and word_count > 200:
        recommendations.append({
            'type': 'structure',
            'severity': 'medium',
            'message': 'Document lacks headers for organization.',
            'suggestion': 'Add section headers to improve readability and navigation.'
        })

    if reading_time_minutes > 10:
        recommendations.append({
            'type': 'readability',
            'severity': 'low',
            'message': f'Long reading time ({reading_time_minutes} minutes).',
            'suggestion': 'Consider adding a summary or table of contents.'
        })

    if word_count > 500 and link_count == 0:
        recommendations.append({
            'type': 'engagement',
            'severity': 'low',
            'message': 'Document has no external links.',
            'suggestion': 'Consider adding relevant links to external resources.'
        })

    return recommendations
