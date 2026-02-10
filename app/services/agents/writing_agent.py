"""Writing Agent for content generation and editing."""
from typing import Dict, Any, Optional, List
import logging
import re

from . import register_agent
from .base_agent import BaseAgent, AgentTask, AgentType, TaskStatus

logger = logging.getLogger(__name__)


@register_agent('writing')
class WritingAgent(BaseAgent):
    """Agent specialized for content writing and editing.

    Capabilities:
    - Generate content (articles, summaries, documentation)
    - Edit and improve existing text
    - Adapt tone and style
    - Expand or condense content
    - Translate between languages
    """

    SYSTEM_PROMPT = """You are a Writing Agent specialized in content creation and editing.

Your capabilities:
1. GENERATE: Create new content from scratch based on requirements
2. EDIT: Improve existing text for clarity, grammar, and style
3. REWRITE: Adapt content for different audiences or purposes
4. EXPAND: Elaborate on brief content with more detail
5. CONDENSE: Summarize lengthy content while preserving key information

Guidelines:
- Match the requested tone and style
- Maintain consistency in voice throughout
- Use clear, engaging language
- Structure content logically
- Proofread for errors

Output Format:
- Provide the requested content directly
- Include explanations only when asked
- Use appropriate formatting (headers, lists, etc.) for the content type
"""

    @property
    def agent_type(self) -> AgentType:
        """Return writing agent type."""
        return AgentType.WRITING

    def get_system_prompt(self) -> str:
        """Return the writing agent system prompt."""
        return self.SYSTEM_PROMPT

    def execute(self, task: AgentTask) -> AgentTask:
        """Execute a writing task.

        Supported input_data keys:
        - content: Existing content to work with (optional)
        - task_type: 'generate', 'edit', 'rewrite', 'expand', 'condense'
        - instructions: Specific instructions for the task
        - tone: Desired tone (professional, casual, academic, etc.)
        - format: Output format (article, bullet points, etc.)
        - length: Target length (short, medium, long, or word count)

        Returns:
            Updated task with written content
        """
        task.mark_running()

        try:
            input_data = task.input_data
            content = input_data.get('content', '')
            task_type = input_data.get('task_type', 'generate')
            instructions = input_data.get('instructions', '')
            tone = input_data.get('tone', 'professional')
            output_format = input_data.get('format', 'prose')
            length = input_data.get('length', 'medium')
            previous_output = input_data.get('previous_output', {})

            # Build the writing prompt
            prompt = self._build_prompt(
                content, task_type, instructions, tone,
                output_format, length, previous_output
            )

            # Add step for prompt building
            self._add_step(
                action='build_prompt',
                input_data={'task_type': task_type, 'tone': tone},
                reasoning=f"Building {task_type} prompt with {tone} tone"
            )

            # Determine max tokens based on length
            max_tokens = self._get_max_tokens(length)

            # Call LLM
            response = self._call_llm(
                prompt,
                max_tokens=max_tokens,
                temperature=self.config.get('temperature', 0.7)
            )

            # Add step for LLM call
            self._add_step(
                action='llm_call',
                input_data={'prompt_length': len(prompt)},
                output_data={'response_length': len(response.content)},
                reasoning="Generating content with LLM"
            )

            # Structure the result
            result = {
                'task_type': task_type,
                'content': response.content,
                'word_count': len(response.content.split()),
                'tone': tone,
                'format': output_format,
                'execution_trace': self.get_execution_trace(),
                'usage': response.usage
            }

            task.mark_completed(result)

        except Exception as e:
            logger.error(f"Writing agent execution failed: {e}")
            task.mark_failed(str(e))

        return task

    def _build_prompt(
        self,
        content: str,
        task_type: str,
        instructions: str,
        tone: str,
        output_format: str,
        length: str,
        previous_output: Dict[str, Any]
    ) -> str:
        """Build the writing prompt based on task type."""
        length_guide = {
            'short': '150-300 words',
            'medium': '400-800 words',
            'long': '1000-2000 words'
        }.get(length, length if isinstance(length, str) and length.isdigit() else '400-800 words')

        base_instructions = f"""
Tone: {tone}
Format: {output_format}
Target Length: {length_guide}
{f'Specific Instructions: {instructions}' if instructions else ''}
{f'Previous Context: {previous_output.get("content", "")[:500]}' if previous_output else ''}
"""

        prompts = {
            'generate': f"""Generate new content based on the following requirements:

{base_instructions}

Topic/Requirements:
{instructions or content or 'Create engaging content on the given topic.'}

Write the content directly without preamble or explanation.
""",
            'edit': f"""Edit and improve the following content for clarity, grammar, and style.

{base_instructions}

Original Content:
{content}

Provide the edited version directly. Maintain the original meaning while improving quality.
""",
            'rewrite': f"""Rewrite the following content to match the specified tone and purpose.

{base_instructions}

Original Content:
{content}

Rewrite completely while preserving the core message.
""",
            'expand': f"""Expand the following content with more detail and depth.

{base_instructions}

Content to Expand:
{content}

Add relevant details, examples, and explanations while maintaining coherence.
""",
            'condense': f"""Condense the following content while preserving key information.

{base_instructions}

Content to Condense:
{content}

Create a shorter version that captures all essential points.
"""
        }

        return prompts.get(task_type, prompts['generate'])

    def _get_max_tokens(self, length: str) -> int:
        """Get max tokens based on target length."""
        length_tokens = {
            'short': 500,
            'medium': 1200,
            'long': 3000
        }

        if length in length_tokens:
            return length_tokens[length]

        # If length is a number (word count), estimate tokens
        try:
            words = int(length)
            return int(words * 1.5)  # Rough token estimate
        except (ValueError, TypeError):
            return 1200  # Default to medium
