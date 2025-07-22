"""
AI Service for content suggestions and auto-completion
Provides AI-powered writing assistance capabilities
"""

import openai
import os
from typing import List, Dict, Optional
from flask import current_app
from app.models.document import Document
from app.models.tag import Tag
import re
import logging

logger = logging.getLogger(__name__)

class AIService:
    def __init__(self):
        self.api_key = os.getenv('OPENAI_API_KEY')
        if self.api_key:
            openai.api_key = self.api_key
        self.enabled = bool(self.api_key)
        self.config = {
            'ocrService': 'tesseract',
            'ocrApiKey': '',
            'llmProvider': 'openai',
            'llmApiKey': self.api_key or '',
            'llmModel': 'gpt-3.5-turbo',
            'enableAiTags': True,
            'enableAiSummary': False
        }
    
    def is_enabled(self) -> bool:
        """Check if AI service is enabled"""
        return self.enabled
    
    def get_content_suggestions(self, content: str, cursor_position: int = None, max_suggestions: int = 3) -> List[Dict]:
        """
        Get AI-powered content suggestions based on current document content
        
        Args:
            content: Current document content
            cursor_position: Current cursor position in the document
            max_suggestions: Maximum number of suggestions to return
            
        Returns:
            List of suggestion dictionaries with text and type
        """
        if not self.is_enabled():
            return []
        
        try:
            # Extract context around cursor position
            context = self._extract_context(content, cursor_position)
            
            # Generate suggestions based on context
            suggestions = []
            
            # Completion suggestions
            completion_suggestions = self._get_completion_suggestions(context, max_suggestions)
            suggestions.extend(completion_suggestions)
            
            # Improvement suggestions
            improvement_suggestions = self._get_improvement_suggestions(context)
            suggestions.extend(improvement_suggestions)
            
            return suggestions[:max_suggestions]
            
        except Exception as e:
            logger.error(f"Error getting content suggestions: {e}")
            return []
    
    def get_auto_completion(self, content: str, cursor_position: int) -> Optional[str]:
        """
        Get auto-completion suggestion for current typing position
        
        Args:
            content: Current document content
            cursor_position: Current cursor position
            
        Returns:
            Auto-completion text or None
        """
        if not self.is_enabled():
            return None
        
        try:
            # Extract the current line and partial word
            lines = content[:cursor_position].split('\n')
            current_line = lines[-1] if lines else ""
            
            # Don't suggest if line is too short or ends with whitespace
            if len(current_line.strip()) < 2 or current_line.endswith(' '):
                return None
            
            # Get context for better suggestions
            context = self._extract_context(content, cursor_position, context_size=200)
            
            prompt = f"""
            Context: {context}
            Current line: {current_line}
            
            Complete the current line with a short, relevant continuation (max 10 words).
            Only return the completion text, nothing else.
            """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=20,
                temperature=0.3,
                stop=['\n', '.', '!', '?']
            )
            
            completion = response.choices[0].text.strip()
            return completion if completion else None
            
        except Exception as e:
            logger.error(f"Error getting auto-completion: {e}")
            return None
    
    def suggest_tags(self, content: str, title: str = "") -> List[str]:
        """
        Suggest relevant tags based on document content
        
        Args:
            content: Document content
            title: Document title
            
        Returns:
            List of suggested tag names
        """
        if not self.is_enabled():
            return self._fallback_tag_suggestions(content, title)
        
        try:
            # Get existing tags for reference
            existing_tags = [tag.name for tag in Tag.query.limit(50).all()]
            
            # Detect the primary language of the content
            content_sample = (title + " " + content[:500]).strip()
            language = self._detect_language(content_sample)
            
            if language == 'korean':
                prompt = f"""
                당신은 옵시디언 노트를 정리하는 AI 어시스턴트입니다.
                아래 마크다운 문서의 핵심 내용을 분석하고 가장 관련성 높은 태그 9개를 한글로 만드세요.

                # 요구사항:
                1. 정확히 9개의 태그를 만들어야 합니다.
                2. 각 태그는 '#' 기호로 시작해야 합니다 (예: #인공지능).
                3. 태그를 공백으로 구분한 한 줄 문자열로 반환하세요.
                4. 태그 문자열만 출력하고, 다른 설명이나 줄바꿈은 없어야 합니다.
                
                문서 제목: {title}
                문서 내용: {content[:1000]}...
                """
            elif language == 'japanese':
                prompt = f"""
                あなたはObsidianノートを整理するAIアシスタントです。
                以下のマークダウン文書の核心内容を分析し、最も関連性の高いタグ9個を日本語で作成してください。

                # 要件:
                1. 正確に9個のタグを作成する必要があります。
                2. 各タグは'#'記号で始まる必要があります（例：#人工知能）。
                3. タグをスペースで区切った一行の文字列として返してください。
                4. タグ文字列のみを出力し、他の説明や改行はありません。
                
                文書タイトル: {title}
                文書内容: {content[:1000]}...
                """
            else:  # Default to English
                prompt = f"""
                You are an AI assistant helping to organize Obsidian notes.
                Analyze the core content of the markdown document below and create the 9 most relevant tags in English.

                # Requirements:
                1. only 9 tags must be created.
                2. each tag must start with the '#' symbol (e.g. #AI).
                3. Return the tags as a single line of string separated by spaces.
                4. output only the tag string, no other comments or newlines.
                
                Document Title: {title}
                Document Content: {content[:1000]}...
                """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=50,
                temperature=0.2
            )
            
            response_text = response.choices[0].text.strip()
            # Split by spaces and remove '#' symbols
            suggested_tags = [tag.strip().lstrip('#') for tag in response_text.split()]
            return [tag for tag in suggested_tags if tag and len(tag) < 50]
            
        except Exception as e:
            logger.error(f"Error getting tag suggestions: {e}")
            return self._fallback_tag_suggestions(content, title)
    
    def suggest_title(self, content: str) -> Optional[str]:
        """
        Suggest a title based on document content
        
        Args:
            content: Document content
            
        Returns:
            Suggested title or None
        """
        if not self.is_enabled():
            return self._fallback_title_suggestion(content)
        
        try:
            # Use first few paragraphs for title suggestion
            content_preview = content[:500]
            
            prompt = f"""
            Document content: {content_preview}
            
            Suggest a concise, descriptive title for this document (max 8 words).
            Return only the title, nothing else.
            """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=20,
                temperature=0.3
            )
            
            title = response.choices[0].text.strip()
            return title if title and len(title) < 100 else None
            
        except Exception as e:
            logger.error(f"Error getting title suggestion: {e}")
            return self._fallback_title_suggestion(content)
    
    def get_writing_suggestions(self, content: str) -> List[Dict]:
        """
        Get writing improvement suggestions
        
        Args:
            content: Document content to analyze
            
        Returns:
            List of improvement suggestions
        """
        if not self.is_enabled():
            return []
        
        try:
            prompt = f"""
            Analyze this text and provide 2-3 brief writing improvement suggestions:
            
            {content[:800]}
            
            Focus on:
            - Clarity and readability
            - Structure and organization
            - Grammar and style
            
            Return suggestions as a numbered list.
            """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=150,
                temperature=0.2
            )
            
            suggestions_text = response.choices[0].text.strip()
            suggestions = []
            
            for line in suggestions_text.split('\n'):
                line = line.strip()
                if line and (line[0].isdigit() or line.startswith('-')):
                    # Remove numbering and clean up
                    suggestion = re.sub(r'^\d+\.?\s*', '', line)
                    suggestion = re.sub(r'^-\s*', '', suggestion)
                    if suggestion:
                        suggestions.append({
                            'type': 'improvement',
                            'text': suggestion
                        })
            
            return suggestions
            
        except Exception as e:
            logger.error(f"Error getting writing suggestions: {e}")
            return []
    
    def _extract_context(self, content: str, cursor_position: int = None, context_size: int = 300) -> str:
        """Extract relevant context around cursor position"""
        if cursor_position is None:
            cursor_position = len(content)
        
        start = max(0, cursor_position - context_size // 2)
        end = min(len(content), cursor_position + context_size // 2)
        
        return content[start:end]
    
    def _get_completion_suggestions(self, context: str, max_suggestions: int) -> List[Dict]:
        """Get completion suggestions using AI"""
        try:
            prompt = f"""
            Context: {context}
            
            Suggest {max_suggestions} possible ways to continue this text.
            Keep suggestions brief and relevant.
            Return each suggestion on a new line.
            """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=100,
                temperature=0.4
            )
            
            suggestions = []
            for line in response.choices[0].text.strip().split('\n'):
                line = line.strip()
                if line:
                    suggestions.append({
                        'type': 'completion',
                        'text': line
                    })
            
            return suggestions
            
        except Exception as e:
            logger.error(f"Error getting completion suggestions: {e}")
            return []
    
    def _get_improvement_suggestions(self, context: str) -> List[Dict]:
        """Get improvement suggestions for the current context"""
        try:
            if len(context.strip()) < 50:
                return []
            
            prompt = f"""
            Text: {context}
            
            Suggest one brief improvement for clarity or style.
            Return only the suggestion, nothing else.
            """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=50,
                temperature=0.2
            )
            
            suggestion = response.choices[0].text.strip()
            if suggestion:
                return [{
                    'type': 'improvement',
                    'text': suggestion
                }]
            
            return []
            
        except Exception as e:
            logger.error(f"Error getting improvement suggestions: {e}")
            return []
    
    def _fallback_tag_suggestions(self, content: str, title: str) -> List[str]:
        """Fallback tag suggestions without AI"""
        tags = []
        text = (title + " " + content).lower()
        
        # Detect language for appropriate tag suggestions
        language = self._detect_language(text)
        
        # Language-specific keyword-based tag suggestions
        if language == 'korean':
            keyword_tags = {
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
                
                # 미디어 (일반적인 "뉴스" 태그 피하기)
                'newsbreak': ['미디어'],
                '뉴스': ['미디어'],
                'breaking': ['긴급'],
            }
        elif language == 'japanese':
            keyword_tags = {
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
        else:  # English
            keyword_tags = {
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
                
                # Media (avoid generic "news" tags)
                'newsbreak': ['media'],
                'breaking': ['urgent'],
            }
        
        # Check for keywords in text (but exclude URLs, file extensions, etc.)
        import re
        # Remove URLs and file paths to avoid false matches
        cleaned_text = re.sub(r'https?://[^\s]+', ' ', text)  # Remove URLs
        cleaned_text = re.sub(r'[^\s]+\.(php|js|css|html|jpg|png|gif|pdf|doc|docx)[^\s]*', ' ', cleaned_text)  # Remove file extensions
        cleaned_text = re.sub(r'/[^\s]*', ' ', cleaned_text)  # Remove file paths
        
        # Also check title separately for better keyword matching
        title_and_content = title.lower() + " " + cleaned_text
        
        # Only do keyword matching if we have substantial content
        if len(cleaned_text.strip()) > 50:  # At least 50 characters of meaningful content
            for keyword, suggested_tags in keyword_tags.items():
                # Use word boundary matching to avoid partial matches, include both content and title
                if re.search(r'\b' + re.escape(keyword) + r'\b', title_and_content, re.IGNORECASE):
                    tags.extend(suggested_tags)
        
        # If no keywords found, try to extract meaningful terms from content
        if not tags:
            # Look for meaningful words, but be more selective
            words = cleaned_text.split()
            potential_tags = []
            
            # Common stop words and generic terms to exclude
            stop_words = {
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
            
            for word in words:
                word = word.strip('.,!?()[]{}":;').lower()
                # Only consider words that are:
                # - At least 4 characters long
                # - Alphabetic (no numbers/symbols)
                # - Not common stop words
                # - Not too common (appear less than 5 times to avoid repetitive words)
                if (len(word) >= 4 and 
                    word.isalpha() and 
                    word not in stop_words and
                    words.count(word) < 5):
                    potential_tags.append(word)
            
            # Get unique potential tags and limit to most promising ones
            unique_tags = list(set(potential_tags))
            
            # Prefer longer, more specific words
            unique_tags.sort(key=len, reverse=True)
            
            # Only add the most meaningful words as tags (max 2)
            for word in unique_tags[:2]:
                if len(word) >= 5:  # Only longer, more specific words
                    tags.append(word)
        
        # Don't add any generic fallback tags - better to have fewer meaningful tags
        # Remove duplicates and limit to most relevant tags
        unique_tags = list(set(tags))
        
        # If we have fewer than 9 tags, that's okay - quality over quantity
        return unique_tags[:9]
    
    def _detect_language(self, text: str) -> str:
        """Detect the primary language of the text"""
        import re
        
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
        english_ratio = english_chars / total_chars
        
        if korean_ratio > 0.4:
            return 'korean'
        elif japanese_ratio > 0.4:
            return 'japanese'
        else:
            return 'english'
    
    def _fallback_title_suggestion(self, content: str) -> Optional[str]:
        """Fallback title suggestion without AI"""
        lines = content.strip().split('\n')
        
        # Look for first non-empty line that looks like a title
        for line in lines[:5]:
            line = line.strip()
            if line and not line.startswith('#') and len(line) < 100:
                # Clean up and return first sentence or phrase
                title = line.split('.')[0].strip()
                if 5 < len(title) < 80:
                    return title
        
        return None
    
    def save_config(self, config_data: Dict) -> bool:
        """
        Save AI configuration settings
        
        Args:
            config_data: Dictionary containing AI configuration
            
        Returns:
            True if configuration was saved successfully
        """
        try:
            # Update internal config
            self.config.update(config_data)
            
            # Update OpenAI API key if changed
            if 'llmApiKey' in config_data and config_data['llmProvider'] == 'openai':
                self.api_key = config_data['llmApiKey']
                if self.api_key:
                    openai.api_key = self.api_key
                    self.enabled = True
                else:
                    self.enabled = False
            
            logger.info(f"AI configuration saved: {config_data.get('llmProvider', 'unknown')} provider")
            return True
            
        except Exception as e:
            logger.error(f"Error saving AI configuration: {e}")
            return False
    
    def test_connection(self, service: str, config_data: Dict) -> Dict:
        """
        Test connection to AI service
        
        Args:
            service: Service to test ('ocr' or 'llm')
            config_data: Configuration data for testing
            
        Returns:
            Dictionary with success status and error message if applicable
        """
        try:
            if service == 'llm':
                return self._test_llm_connection(config_data)
            elif service == 'ocr':
                return self._test_ocr_connection(config_data)
            else:
                return {
                    'success': False,
                    'error': f'Unknown service: {service}'
                }
        except Exception as e:
            logger.error(f"Error testing {service} connection: {e}")
            return {
                'success': False,
                'error': f'Connection test failed: {str(e)}'
            }
    
    def _test_llm_connection(self, config_data: Dict) -> Dict:
        """Test LLM provider connection"""
        try:
            provider = config_data.get('llmProvider', 'openai')
            api_key = config_data.get('llmApiKey', '')
            
            if not api_key:
                return {
                    'success': False,
                    'error': 'API key is required'
                }
            
            if provider == 'openai':
                # For OpenAI, we'll just validate the key format for now
                # since the old API is deprecated and we'd need to update the entire service
                if not api_key.startswith(('sk-', 'sk-proj-')):
                    return {
                        'success': False,
                        'error': 'Invalid OpenAI API key format. Keys should start with "sk-"'
                    }
                
                if len(api_key) < 20:
                    return {
                        'success': False,
                        'error': 'OpenAI API key appears to be too short'
                    }
                
                # Basic format validation passed
                return {
                    'success': True,
                    'message': 'OpenAI API key format is valid (connection not fully tested due to API version compatibility)'
                }
            
            else:
                # For other providers, just validate the key format
                if len(api_key) < 10:
                    return {
                        'success': False,
                        'error': 'API key appears to be invalid'
                    }
                return {
                    'success': True,
                    'note': f'{provider} connection not fully tested'
                }
                
        except Exception as e:
            return {
                'success': False,
                'error': f'LLM test failed: {str(e)}'
            }
    
    def _test_ocr_connection(self, config_data: Dict) -> Dict:
        """Test OCR service connection"""
        try:
            service = config_data.get('ocrService', 'tesseract')
            
            if service == 'tesseract':
                # Tesseract is local, always available
                return {'success': True}
            
            api_key = config_data.get('ocrApiKey', '')
            if not api_key:
                return {
                    'success': False,
                    'error': 'API key is required for cloud OCR services'
                }
            
            # For cloud services, validate key format
            if len(api_key) < 10:
                return {
                    'success': False,
                    'error': 'API key appears to be invalid'
                }
            
            return {
                'success': True,
                'note': f'{service} connection not fully tested'
            }
            
        except Exception as e:
            return {
                'success': False,
                'error': f'OCR test failed: {str(e)}'
            }

# Global AI service instance
ai_service = AIService()