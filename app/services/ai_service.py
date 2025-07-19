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
            
            prompt = f"""
            Document Title: {title}
            Document Content: {content[:1000]}...
            
            Existing tags in system: {', '.join(existing_tags[:20])}
            
            Suggest 3-5 relevant tags for this document. 
            Prefer existing tags when applicable, but suggest new ones if needed.
            Return only tag names separated by commas.
            """
            
            response = openai.Completion.create(
                engine="text-davinci-003",
                prompt=prompt,
                max_tokens=50,
                temperature=0.2
            )
            
            suggested_tags = [tag.strip() for tag in response.choices[0].text.split(',')]
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
        
        # Simple keyword-based tag suggestions
        keyword_tags = {
            'python': ['python', 'programming'],
            'javascript': ['javascript', 'web'],
            'react': ['react', 'frontend'],
            'api': ['api', 'backend'],
            'database': ['database', 'data'],
            'tutorial': ['tutorial', 'guide'],
            'project': ['project', 'development']
        }
        
        for keyword, suggested_tags in keyword_tags.items():
            if keyword in text:
                tags.extend(suggested_tags)
        
        return list(set(tags))[:5]
    
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

# Global AI service instance
ai_service = AIService()