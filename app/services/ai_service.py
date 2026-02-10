"""
AI Service for content suggestions and auto-completion
Provides AI-powered writing assistance capabilities
"""

from openai import OpenAI
import os
from typing import List, Dict, Optional, Any
from app.models.ai_config import AIConfig
from app import db
import re
import logging

from app.services.ai_tag_suggestion import (
    fallback_tag_suggestions,
    fallback_title_suggestion,
)
from app.services.ai_config_service import (
    get_env_keys,
    get_default_config,
    get_safe_config,
    test_llm_connection,
    test_ocr_connection,
)

logger = logging.getLogger(__name__)


class AIService:
    def __init__(self):
        app_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        self.config_file = os.path.join(app_dir, 'ai_config.json')

        self.env_keys = get_env_keys()
        logger.info(f"Initializing AIService with env keys: {[k for k, v in self.env_keys.items() if v]}")

        self.config = self._load_config()

        llm_provider = self.config.get('llmProvider', 'openai')
        llm_api_key = self.config.get('llmApiKey', '')
        self.api_key = llm_api_key

        self.openai_client: Optional[OpenAI] = None
        if llm_provider == 'openai' and llm_api_key:
            self.openai_client = OpenAI(api_key=llm_api_key)
            self.enabled = True
        elif llm_api_key:
            self.enabled = True
        else:
            self.enabled = False

        logger.info(f"AIService initialized: provider={llm_provider}, enabled={self.enabled}")

    def _load_config(self) -> Dict:
        """Load configuration from database with environment variable fallbacks"""
        default_config = get_default_config(self.env_keys)

        try:
            logger.info("Attempting to load AI config from database")
            self._ensure_table_exists()

            config = {}
            for key in default_config.keys():
                value = AIConfig.get_value(key)
                if value is not None:
                    if key in ['enableAiTags', 'enableAiSummary']:
                        config[key] = value.lower() == 'true'
                    else:
                        config[key] = value

            if config:
                logger.info(f"Successfully loaded config from database")
                default_config.update(config)
            else:
                logger.info("No AI configuration found in database, using defaults")

            llm_provider = default_config.get('llmProvider', 'openai')
            if not default_config.get('llmApiKey') and self.env_keys.get(llm_provider):
                default_config['llmApiKey'] = self.env_keys[llm_provider]
                logger.info(f"Applied {llm_provider} API key from environment variables")

        except Exception as e:
            logger.error(f"Error loading AI configuration from database: {e}, using defaults")

        return default_config

    def _ensure_table_exists(self):
        """Ensure the AI config table exists"""
        try:
            from sqlalchemy import text
            with db.engine.connect() as conn:
                conn.execute(text("""
                    CREATE TABLE IF NOT EXISTS ai_config (
                        id SERIAL PRIMARY KEY,
                        key VARCHAR(50) UNIQUE NOT NULL,
                        value TEXT,
                        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                    )
                """))
                conn.commit()
            logger.info("AI config table ensured to exist")
        except Exception as e:
            logger.error(f"Error ensuring AI config table exists: {e}")

    def _save_config(self) -> bool:
        """Save current configuration to database"""
        try:
            logger.info("Attempting to save AI config to database")
            self._ensure_table_exists()

            success = True
            for key, value in self.config.items():
                if isinstance(value, bool):
                    str_value = str(value).lower()
                else:
                    str_value = str(value) if value is not None else ''

                if not AIConfig.set_value(key, str_value):
                    logger.error(f"Failed to save config key: {key}")
                    success = False

            if success:
                logger.info("Successfully saved AI configuration to database")
            return success
        except Exception as e:
            logger.error(f"Error saving AI configuration to database: {e}")
            return False

    def is_enabled(self) -> bool:
        """Check if AI service is enabled"""
        return bool(self.enabled)

    def get_content_suggestions(self, content: str, cursor_position: Optional[int] = None,
                                 max_suggestions: int = 3) -> List[Dict[str, Any]]:
        """Get AI-powered content suggestions based on current document content"""
        if not self.is_enabled():
            return []

        try:
            context = self._extract_context(content, cursor_position)
            suggestions = []

            completion_suggestions = self._get_completion_suggestions(context, max_suggestions)
            suggestions.extend(completion_suggestions)

            improvement_suggestions = self._get_improvement_suggestions(context)
            suggestions.extend(improvement_suggestions)

            return suggestions[:max_suggestions]

        except Exception as e:
            logger.error(f"Error getting content suggestions: {e}")
            return []

    def get_auto_completion(self, content: str, cursor_position: int) -> Optional[str]:
        """Get auto-completion suggestion for current typing position"""
        if not self.is_enabled():
            return None

        try:
            lines = content[:cursor_position].split('\n')
            current_line = lines[-1] if lines else ""

            if len(current_line.strip()) < 2 or current_line.endswith(' '):
                return None

            context = self._extract_context(content, cursor_position, context_size=200)

            prompt = f"""
            Context: {context}
            Current line: {current_line}

            Complete the current line with a short, relevant continuation (max 10 words).
            Only return the completion text, nothing else.
            """

            if not self.openai_client:
                return None

            response = self.openai_client.chat.completions.create(
                model=self.config.get('llmModel', 'gpt-3.5-turbo'),
                messages=[{"role": "user", "content": prompt}],
                max_tokens=20,
                temperature=0.3,
                stop=['\n', '.', '!', '?']
            )

            completion = (response.choices[0].message.content or '').strip()
            return completion if completion else None

        except Exception as e:
            logger.error(f"Error getting auto-completion: {e}")
            return None

    def suggest_tags(self, content: str, title: str = "") -> List[str]:
        """Suggest relevant tags based on document content"""
        if not self.is_enabled():
            return fallback_tag_suggestions(content, title)

        try:
            prompt = f"""
                You are an AI assistant helping to organize Obsidian notes.
                Analyze the core content of the markdown document below and create the 9 most relevant tags in English.

                # Requirements:
                1. only 9 tags must be created.
                2. each tag must start with the '#' symbol (e.g. #AI).
                3. Return the tags as a single line of string separated by spaces.
                4. output only the tag string, no other comments or newlines.

                Document Title: {title}
                Document Content: {content[:3000]}...
                """

            if not self.openai_client:
                return fallback_tag_suggestions(content, title)

            response = self.openai_client.chat.completions.create(
                model=self.config.get('llmModel', 'gpt-3.5-turbo'),
                messages=[{"role": "user", "content": prompt}],
                max_tokens=50,
                temperature=0.2
            )

            response_text = (response.choices[0].message.content or '').strip()
            suggested_tags = [tag.strip().lstrip('#') for tag in response_text.split()]
            return [tag for tag in suggested_tags if tag and len(tag) < 50]

        except Exception as e:
            logger.error(f"Error getting tag suggestions: {e}")
            return fallback_tag_suggestions(content, title)

    def suggest_title(self, content: str) -> Optional[str]:
        """Suggest a title based on document content"""
        if not self.is_enabled():
            return fallback_title_suggestion(content)

        try:
            content_preview = content[:1000]

            prompt = f"""
            Document content: {content_preview}

            Suggest a concise, descriptive title for this document (max 3 lines ).
            Return only the title, nothing else.
            """

            if not self.openai_client:
                return fallback_title_suggestion(content)

            response = self.openai_client.chat.completions.create(
                model=self.config.get('llmModel', 'gpt-3.5-turbo'),
                messages=[{"role": "user", "content": prompt}],
                max_tokens=20,
                temperature=0.3
            )

            title = (response.choices[0].message.content or '').strip()
            return title if title and len(title) < 100 else None

        except Exception as e:
            logger.error(f"Error getting title suggestion: {e}")
            return fallback_title_suggestion(content)

    def get_writing_suggestions(self, content: str) -> List[Dict]:
        """Get writing improvement suggestions"""
        if not self.is_enabled():
            return []

        try:
            prompt = f"""
            Analyze this text and provide 2-3 brief writing improvement suggestions:

            {content[:1200]}

            Focus on:
            - Clarity and readability
            - Structure and organization
            - Grammar and style

            Return suggestions as a numbered list.
            """

            if not self.openai_client:
                return []

            response = self.openai_client.chat.completions.create(
                model=self.config.get('llmModel', 'gpt-3.5-turbo'),
                messages=[{"role": "user", "content": prompt}],
                max_tokens=150,
                temperature=0.2
            )

            suggestions_text = (response.choices[0].message.content or '').strip()
            suggestions = []

            for line in suggestions_text.split('\n'):
                line = line.strip()
                if line and (line[0].isdigit() or line.startswith('-')):
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

    def _extract_context(self, content: str, cursor_position: Optional[int] = None,
                         context_size: int = 300) -> str:
        """Extract relevant context around cursor position"""
        if cursor_position is None:
            cursor_position = len(content)

        start = max(0, cursor_position - context_size // 2)
        end = min(len(content), cursor_position + context_size // 2)

        return content[start:end]

    def _get_completion_suggestions(self, context: str, max_suggestions: int) -> List[Dict[str, Any]]:
        """Get completion suggestions using AI"""
        try:
            prompt = f"""
            Context: {context}

            Suggest {max_suggestions} possible ways to continue this text.
            Keep suggestions brief and relevant.
            Return each suggestion on a new line.
            """

            if not self.openai_client:
                return []

            response = self.openai_client.chat.completions.create(
                model=self.config.get('llmModel', 'gpt-3.5-turbo'),
                messages=[{"role": "user", "content": prompt}],
                max_tokens=100,
                temperature=0.4
            )

            suggestions: List[Dict[str, Any]] = []
            for line in (response.choices[0].message.content or '').strip().split('\n'):
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

    def _get_improvement_suggestions(self, context: str) -> List[Dict[str, Any]]:
        """Get improvement suggestions for the current context"""
        try:
            if len(context.strip()) < 50:
                return []

            prompt = f"""
            Text: {context}

            Suggest one brief improvement for clarity or style.
            Return only the suggestion, nothing else.
            """

            if not self.openai_client:
                return []

            response = self.openai_client.chat.completions.create(
                model=self.config.get('llmModel', 'gpt-3.5-turbo'),
                messages=[{"role": "user", "content": prompt}],
                max_tokens=50,
                temperature=0.2
            )

            suggestion = (response.choices[0].message.content or '').strip()
            if suggestion:
                return [{
                    'type': 'improvement',
                    'text': suggestion
                }]

            return []

        except Exception as e:
            logger.error(f"Error getting improvement suggestions: {e}")
            return []

    def get_config(self) -> Dict[str, Any]:
        """Get current AI configuration settings (with masked API keys)"""
        return get_safe_config(self.config)

    def save_config(self, config_data: Dict) -> bool:
        """Save AI configuration settings"""
        try:
            config_data = config_data.copy()

            llm_api_key = config_data.get('llmApiKey', '')
            if llm_api_key and '*' in llm_api_key:
                config_data['llmApiKey'] = self.config.get('llmApiKey', '')
                llm_api_key = config_data['llmApiKey']

            ocr_api_key = config_data.get('ocrApiKey', '')
            if ocr_api_key and '*' in ocr_api_key:
                config_data['ocrApiKey'] = self.config.get('ocrApiKey', '')

            self.config.update(config_data)

            llm_provider = config_data.get('llmProvider', 'openai')

            if llm_provider == 'openai' and llm_api_key:
                self.api_key = llm_api_key
                self.openai_client = OpenAI(api_key=llm_api_key)
                self.enabled = True
            elif llm_api_key:
                self.api_key = llm_api_key
                self.enabled = True
            else:
                self.enabled = False

            file_saved = self._save_config()

            logger.info(f"AI configuration saved: {llm_provider} provider, file_saved: {file_saved}")
            return file_saved

        except Exception as e:
            logger.error(f"Error saving AI configuration: {e}")
            return False

    def test_connection(self, service: str, config_data: Dict) -> Dict:
        """Test connection to AI service"""
        try:
            if service == 'llm':
                return test_llm_connection(config_data)
            elif service == 'ocr':
                return test_ocr_connection(config_data)
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


# Global AI service instance
ai_service = AIService()
