"""
AI Tag Suggestion Service
Provides fallback tag suggestions based on keyword matching and language detection
"""

import re
from typing import List, Optional


def detect_language(text: str) -> str:
    """Detect the primary language of the text"""
    # Count characters by language
    korean_chars = len(re.findall(r'[가-힣]', text))
    japanese_chars = len(re.findall(r'[ひ-ゞァ-ヾ一-龯]', text))
    english_chars = len(re.findall(r'[a-zA-Z]', text))

    total_chars = korean_chars + japanese_chars + english_chars
    if total_chars == 0:
        return 'english'  # Default

    # Determine primary language (>40% threshold)
    korean_ratio = korean_chars / total_chars
    japanese_ratio = japanese_chars / total_chars

    if korean_ratio > 0.4:
        return 'korean'
    elif japanese_ratio > 0.4:
        return 'japanese'
    else:
        return 'english'


def get_keyword_tags_korean() -> dict:
    """Get Korean keyword-to-tags mapping"""
    return {
        # 프로그래밍 언어
        'python': ['파이썬', '프로그래밍'],
        'javascript': ['자바스크립트', '웹', '프로그래밍'],
        'java': ['자바', '프로그래밍'],
        'react': ['리액트', '프론트엔드', '자바스크립트'],

        # 정치 및 정부
        'trump': ['트럼프', '정치', '정부'],
        '트럼프': ['트럼프', '정치', '정부'],
        'president': ['대통령', '정치'],
        '대통령': ['대통령', '정치'],
        'nasa': ['나사', '우주', '정부'],
        '나사': ['나사', '우주', '정부'],
        'administrator': ['관리자', '정부', '리더십'],
        '관리자': ['관리자', '정부', '리더십'],
        'appointment': ['임명', '정치', '정부'],
        '임명': ['임명', '정치', '정부'],
        '장관': ['장관', '정부'],
        '우주': ['우주'],
        'space': ['우주', '과학'],
        'elon': ['일론머스크', '우주', '기술'],
        'musk': ['일론머스크', '우주', '기술'],
        '머스크': ['일론머스크', '우주'],
        'tesla': ['테슬라', '기술', '전기차'],
        '테슬라': ['테슬라', '기술'],

        # 기술 및 AI
        'ai': ['인공지능'],
        '인공지능': ['인공지능'],
        'machine learning': ['머신러닝', '인공지능'],
        '머신러닝': ['머신러닝', '인공지능'],
        'deep learning': ['딥러닝', '인공지능'],
        '딥러닝': ['딥러닝', '인공지능'],

        # 금융 및 투자 관련
        'etf': ['ETF', '투자', '금융'],
        'ETF': ['ETF', '투자', '금융'],
        'kodex': ['Kodex', 'ETF', '투자'],
        'Kodex': ['Kodex', 'ETF', '투자'],
        '분배금': ['분배금', '투자', '수익'],
        '배당금': ['배당금', '투자', '수익'],
        '고배당': ['고배당', '투자'],
        '커버드콜': ['커버드콜', '투자전략'],
        '미국주식': ['미국주식', '투자', '해외투자'],
        '연금': ['연금', '금융', '보험'],
        '투자': ['투자', '금융'],
        '채권': ['채권', '투자', '금융'],
        '주식': ['주식', '투자'],
        '펀드': ['펀드', '투자'],
        '금융': ['금융'],
        '은행': ['은행', '금융'],
        '보험': ['보험', '금융'],
        '증권': ['증권', '투자'],

        # 미디어
        'newsbreak': ['미디어'],
        '뉴스': ['미디어'],
        'breaking': ['긴급'],
    }


def get_keyword_tags_japanese() -> dict:
    """Get Japanese keyword-to-tags mapping"""
    return {
        # プログラミング言語
        'python': ['パイソン', 'プログラミング'],
        'javascript': ['ジャバスクリプト', 'ウェブ', 'プログラミング'],
        'java': ['ジャバ', 'プログラミング'],
        'react': ['リアクト', 'フロントエンド', 'ジャバスクリプト'],

        # 政治と政府
        'trump': ['トランプ', '政治', '政府'],
        'president': ['大統領', '政治'],
        'nasa': ['ナサ', '宇宙', '政府'],
        'administrator': ['管理者', '政府', 'リーダーシップ'],
        'appointment': ['任命', '政治', '政府'],
        'space': ['宇宙', '科学'],
        'elon': ['イーロンマスク', '宇宙', '技術'],
        'musk': ['イーロンマスク', '宇宙', '技術'],
        'tesla': ['テスラ', '技術', '電気自動車'],

        # 技術とAI
        'ai': ['人工知能'],
        'machine learning': ['機械学習', '人工知能'],
        'deep learning': ['深層学習', '人工知能'],

        # 金融と投資
        'etf': ['ETF', '投資', '金融'],
        'ETF': ['ETF', '投資', '金融'],
        '投資': ['投資', '金融'],
        '金融': ['金融'],
        '株式': ['株式', '投資'],
        '債券': ['債券', '投資'],
        '年金': ['年金', '保険'],

        # メディア
        'newsbreak': ['メディア'],
        'breaking': ['緊急'],
    }


def get_keyword_tags_english() -> dict:
    """Get English keyword-to-tags mapping"""
    return {
        # Programming languages
        'python': ['python', 'programming'],
        'javascript': ['javascript', 'web', 'programming'],
        'java': ['java', 'programming'],
        'react': ['react', 'frontend', 'javascript'],

        # Politics and government
        'trump': ['trump', 'politics', 'government'],
        'president': ['president', 'politics'],
        'nasa': ['nasa', 'space', 'government'],
        'administrator': ['government', 'leadership'],
        'appointment': ['politics', 'government'],
        'space': ['space', 'science'],
        'elon': ['elon-musk', 'space', 'technology'],
        'musk': ['elon-musk', 'space', 'technology'],
        'tesla': ['tesla', 'technology', 'electric-vehicle'],

        # AI and ML
        'ai': ['ai', 'artificial-intelligence'],
        'machine learning': ['machine-learning', 'ai'],
        'deep learning': ['deep-learning', 'ai'],
        'neural network': ['neural-networks', 'ai'],

        # Finance and Investment
        'etf': ['etf', 'investment', 'finance'],
        'ETF': ['etf', 'investment', 'finance'],
        'investment': ['investment', 'finance'],
        'finance': ['finance'],
        'stock': ['stock', 'investment'],
        'stocks': ['stocks', 'investment'],
        'bond': ['bond', 'investment'],
        'bonds': ['bonds', 'investment'],
        'dividend': ['dividend', 'investment'],
        'pension': ['pension', 'retirement'],
        'retirement': ['retirement', 'finance'],
        'fund': ['fund', 'investment'],
        'mutual fund': ['mutual-fund', 'investment'],
        'portfolio': ['portfolio', 'investment'],
        'trading': ['trading', 'investment'],
        'bank': ['bank', 'finance'],
        'insurance': ['insurance', 'finance'],

        # Media
        'newsbreak': ['media'],
        'breaking': ['urgent'],
    }


STOP_WORDS = {
    'the', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by',
    'from', 'up', 'about', 'into', 'through', 'during', 'before', 'after', 'above', 'below',
    'this', 'that', 'these', 'those', 'here', 'there', 'where', 'when', 'why', 'how',
    'what', 'which', 'who', 'whom', 'whose', 'will', 'would', 'should', 'could', 'can',
    'may', 'might', 'must', 'shall', 'have', 'has', 'had', 'do', 'does', 'did', 'is',
    'are', 'was', 'were', 'be', 'been', 'being', 'get', 'got', 'getting', 'go', 'goes',
    'went', 'going', 'come', 'came', 'coming', 'take', 'took', 'taken', 'taking',
    'make', 'made', 'making', 'see', 'saw', 'seen', 'seeing', 'know', 'knew', 'known',
    'think', 'thought', 'thinking', 'say', 'said', 'saying', 'tell', 'told', 'telling',
    'use', 'used', 'using', 'work', 'worked', 'working', 'want', 'wanted', 'wanting',
    'need', 'needed', 'needing', 'try', 'tried', 'trying', 'find', 'found', 'finding',
    'give', 'gave', 'given', 'giving', 'put', 'puts', 'putting', 'set', 'sets', 'setting',
    'article', 'note', 'document', 'content', 'text', 'information', 'data', 'item',
    'thing', 'stuff', 'part', 'way', 'time', 'year', 'day', 'week', 'month', 'hour',
    'minute', 'second', 'today', 'yesterday', 'tomorrow', 'now', 'then', 'soon', 'later',
    'first', 'second', 'third', 'last', 'next', 'previous', 'new', 'old', 'young',
    'good', 'bad', 'best', 'better', 'worse', 'worst', 'big', 'small', 'large', 'little',
    'long', 'short', 'high', 'low', 'great', 'important', 'right', 'wrong', 'true', 'false'
}


def fallback_tag_suggestions(content: str, title: str) -> List[str]:
    """Fallback tag suggestions without AI"""
    tags = []
    text = (title + " " + content).lower()

    # Detect language for appropriate tag suggestions
    language = detect_language(text)

    # Language-specific keyword-based tag suggestions
    if language == 'korean':
        keyword_tags = get_keyword_tags_korean()
    elif language == 'japanese':
        keyword_tags = get_keyword_tags_japanese()
    else:
        keyword_tags = get_keyword_tags_english()

    # Remove URLs and file paths to avoid false matches
    cleaned_text = re.sub(r'https?://[^\s]+', ' ', text)
    cleaned_text = re.sub(r'[^\s]+\.(php|js|css|html|jpg|png|gif|pdf|doc|docx)[^\s]*', ' ', cleaned_text)
    cleaned_text = re.sub(r'/[^\s]*', ' ', cleaned_text)

    # Also check title separately for better keyword matching
    title_and_content = title.lower() + " " + cleaned_text

    # Only do keyword matching if we have substantial content
    if len(cleaned_text.strip()) > 50:
        for keyword, suggested_tags in keyword_tags.items():
            if re.search(r'\b' + re.escape(keyword) + r'\b', title_and_content, re.IGNORECASE):
                tags.extend(suggested_tags)

    # If no keywords found, try to extract meaningful terms from content
    if not tags:
        words = cleaned_text.split()
        potential_tags = []

        for word in words:
            word = word.strip('.,!?()[]{}":;').lower()
            if (len(word) >= 4 and
                word.isalpha() and
                word not in STOP_WORDS and
                words.count(word) < 5):
                potential_tags.append(word)

        unique_tags = list(set(potential_tags))
        unique_tags.sort(key=len, reverse=True)

        for word in unique_tags[:2]:
            if len(word) >= 5:
                tags.append(word)

    unique_tags = list(set(tags))
    return unique_tags[:9]


def fallback_title_suggestion(content: str) -> Optional[str]:
    """Fallback title suggestion without AI"""
    lines = content.strip().split('\n')

    for line in lines[:5]:
        line = line.strip()
        if line and not line.startswith('#') and len(line) < 100:
            title = line.split('.')[0].strip()
            if 5 < len(title) < 80:
                return title

    return None
