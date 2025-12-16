import re
import unicodedata
from typing import List, Dict, Any
from konlpy.tag import Mecab, Kkma, Komoran
import logging

logger = logging.getLogger(__name__)

class KoreanTextProcessor:
    """한국어 텍스트 처리를 위한 유틸리티 클래스"""
    
    def __init__(self, analyzer='mecab'):
        """
        Initialize Korean text processor
        
        Args:
            analyzer: 'mecab', 'kkma', or 'komoran'
        """
        self.analyzer_name = analyzer
        self.analyzer = None
        self._initialize_analyzer()
    
    def _initialize_analyzer(self):
        """형태소 분석기 초기화"""
        try:
            if self.analyzer_name == 'mecab':
                self.analyzer = Mecab()
                logger.info("MeCab Korean analyzer initialized successfully")
            elif self.analyzer_name == 'kkma':
                self.analyzer = Kkma()
                logger.info("Kkma Korean analyzer initialized successfully")
            elif self.analyzer_name == 'komoran':
                self.analyzer = Komoran()
                logger.info("Komoran Korean analyzer initialized successfully")
            else:
                logger.warning(f"Unknown analyzer: {self.analyzer_name}, using Mecab")
                self.analyzer = Mecab()
                logger.info("MeCab Korean analyzer initialized successfully (fallback)")
        except Exception as e:
            logger.error(f"Failed to initialize {self.analyzer_name}: {e}")
            # Fallback to simple text processing
            self.analyzer = None
    
    def normalize_text(self, text: str) -> str:
        """텍스트 정규화"""
        if not text:
            return ""
        
        # Unicode 정규화 (NFC)
        text = unicodedata.normalize('NFC', text)
        
        # 연속된 공백 제거
        text = re.sub(r'\s+', ' ', text)
        
        # 불필요한 특수문자 제거 (한글, 영문, 숫자, 기본 문장부호만 유지)
        text = re.sub(r'[^\w\s\u3130-\u318F\uAC00-\uD7AF.,!?;:\-\[\](){}"\']', ' ', text)
        
        return text.strip()
    
    def tokenize(self, text: str) -> List[str]:
        """텍스트를 토큰으로 분리"""
        if not text:
            return []
        
        normalized_text = self.normalize_text(text)
        
        if self.analyzer is None:
            # 기본 토큰화 (공백 기준)
            return [token for token in normalized_text.split() if len(token) > 1]
        
        try:
            # 형태소 분석을 통한 토큰화
            tokens = self.analyzer.morphs(normalized_text)
            # 길이가 1인 토큰과 조사, 어미 등 제외
            meaningful_tokens = []
            
            for token in tokens:
                if len(token) > 1 and self._is_meaningful_token(token):
                    meaningful_tokens.append(token)
            
            return meaningful_tokens
        except Exception as e:
            logger.error(f"Tokenization failed: {e}")
            # Fallback to simple tokenization
            return [token for token in normalized_text.split() if len(token) > 1]
    
    def extract_keywords(self, text: str, min_length: int = 2, max_keywords: int = 50) -> List[Dict[str, Any]]:
        """키워드 추출 (품사 태깅 포함)"""
        if not text or self.analyzer is None:
            return []
        
        try:
            normalized_text = self.normalize_text(text)
            pos_tags = self.analyzer.pos(normalized_text)
            
            keywords: List[Dict[str, Any]] = []
            keyword_counts: Dict[str, Dict[str, Any]] = {}
            
            for word, pos in pos_tags:
                # 명사, 형용사, 동사 어간만 키워드로 추출
                if (len(word) >= min_length and 
                    pos in ['NNG', 'NNP', 'NNB', 'VA', 'VV', 'MAG'] and
                    self._is_meaningful_token(word)):
                    
                    if word in keyword_counts:
                        keyword_counts[word]['count'] += 1
                    else:
                        keyword_counts[word] = {
                            'word': word,
                            'pos': pos,
                            'count': 1
                        }
            
            # 빈도순으로 정렬
            keywords = sorted(keyword_counts.values(), 
                            key=lambda x: x['count'], reverse=True)
            
            return keywords[:max_keywords]
            
        except Exception as e:
            logger.error(f"Keyword extraction failed: {e}")
            return []
    
    def _is_meaningful_token(self, token: str) -> bool:
        """의미있는 토큰인지 판단"""
        # 숫자만 있는 토큰 제외
        if token.isdigit():
            return False
        
        # 특수문자만 있는 토큰 제외
        if re.match(r'^[^\w\u3130-\u318F\uAC00-\uD7AF]+$', token):
            return False
        
        # 불용어 제외
        stopwords = {
            '이', '그', '저', '것', '수', '등', '및', '또는', '그리고', '하지만',
            '때문', '위해', '통해', '대해', '관해', '에서', '에게', '에게서',
            '으로', '로서', '로써', '에서도', '마저', '조차', '까지', '부터',
            '의해', '에', '을', '를', '이', '가', '은', '는', '과', '와',
            '도', '만', '뿐', '이나', '나', '든지', '거나'
        }
        
        return token not in stopwords
    
    def create_search_vector(self, title: str, content: str) -> str:
        """검색용 벡터 생성"""
        # 제목에 가중치 부여
        title_tokens = self.tokenize(title)
        content_tokens = self.tokenize(content)
        
        # 제목의 토큰은 3번 반복하여 가중치 부여
        weighted_tokens = title_tokens * 3 + content_tokens
        
        return ' '.join(weighted_tokens)
    
    def highlight_korean_text(self, text: str, query: str, max_length: int = 200) -> str:
        """한국어 텍스트에서 검색어 하이라이트"""
        if not query or not text:
            return text[:max_length] + '...' if len(text) > max_length else text
        
        # 검색어 토큰화
        query_tokens = self.tokenize(query.lower())
        if not query_tokens:
            return text[:max_length] + '...' if len(text) > max_length else text
        
        # 텍스트를 문장 단위로 분리
        sentences = re.split(r'[.!?。！？]', text)
        
        best_sentence = ""
        max_matches = 0
        
        for sentence in sentences:
            if len(sentence.strip()) < 10:
                continue
            
            sentence_lower = sentence.lower()
            matches = sum(1 for token in query_tokens if token in sentence_lower)
            
            if matches > max_matches:
                max_matches = matches
                best_sentence = sentence.strip()
        
        if not best_sentence:
            best_sentence = text[:max_length]
        elif len(best_sentence) > max_length:
            best_sentence = best_sentence[:max_length] + '...'
        
        # 검색어 하이라이트
        for token in query_tokens:
            pattern = re.compile(re.escape(token), re.IGNORECASE)
            best_sentence = pattern.sub(f'<mark>{token}</mark>', best_sentence)
        
        return best_sentence
    
    def extract_tags_from_korean_text(self, text: str, max_tags: int = 10) -> List[str]:
        """한국어 텍스트에서 자동 태그 추출"""
        keywords = self.extract_keywords(text, min_length=2, max_keywords=max_tags * 2)
        
        # 명사 우선으로 태그 선별
        tags = []
        for keyword in keywords:
            if keyword['pos'] in ['NNG', 'NNP'] and keyword['count'] >= 2:
                tags.append(keyword['word'])
                if len(tags) >= max_tags:
                    break
        
        return tags
    
    @staticmethod
    def detect_language(text: str) -> str:
        """텍스트의 언어 감지 (한국어/영어/기타)"""
        if not text:
            return 'unknown'
        
        # 한글 문자 비율 계산
        korean_chars = len(re.findall(r'[\uAC00-\uD7AF]', text))
        english_chars = len(re.findall(r'[a-zA-Z]', text))
        total_chars = len(re.findall(r'[\w\uAC00-\uD7AF]', text))
        
        if total_chars == 0:
            return 'unknown'
        
        korean_ratio = korean_chars / total_chars
        english_ratio = english_chars / total_chars
        
        if korean_ratio > 0.3:
            return 'korean'
        elif english_ratio > 0.7:
            return 'english'
        else:
            return 'mixed'

# 전역 인스턴스
korean_processor = KoreanTextProcessor()

def process_korean_document(title: str, content: str) -> Dict[str, Any]:
    """한국어 문서 처리 통합 함수"""
    result = {
        'language': korean_processor.detect_language(content),
        'search_vector': korean_processor.create_search_vector(title, content),
        'keywords': korean_processor.extract_keywords(content),
        'auto_tags': korean_processor.extract_tags_from_korean_text(content),
        'title_tokens': korean_processor.tokenize(title),
        'content_tokens': korean_processor.tokenize(content)
    }
    
    return result

def search_korean_text(documents: List[Dict], query: str, max_results: int = 20) -> List[Dict]:
    """한국어 텍스트 검색"""
    if not query or not documents:
        return []
    
    query_tokens = korean_processor.tokenize(query.lower())
    if not query_tokens:
        return []
    
    scored_docs = []
    
    for doc in documents:
        score = 0
        content_lower = doc.get('content', '').lower()
        title_lower = doc.get('title', '').lower()
        
        # 제목에서 매칭 (가중치 3)
        for token in query_tokens:
            if token in title_lower:
                score += 3
        
        # 내용에서 매칭 (가중치 1)
        for token in query_tokens:
            score += content_lower.count(token)
        
        if score > 0:
            doc_copy = doc.copy()
            doc_copy['search_score'] = score
            doc_copy['highlighted_content'] = korean_processor.highlight_korean_text(
                doc.get('content', ''), query
            )
            scored_docs.append(doc_copy)
    
    # 점수순으로 정렬
    scored_docs.sort(key=lambda x: x['search_score'], reverse=True)
    
    return scored_docs[:max_results]