"""Research Agent for information gathering and analysis."""
from typing import Dict, Any, Optional
import logging
import re

from . import register_agent
from .base_agent import BaseAgent, AgentTask, AgentType, TaskStatus

logger = logging.getLogger(__name__)

# SECURITY: Patterns that may indicate prompt injection attempts
PROMPT_INJECTION_PATTERNS = [
    r'ignore\s+(previous|above|all)\s+instructions',
    r'disregard\s+(previous|above|all)',
    r'forget\s+(previous|above|all)',
    r'system\s*:\s*',
    r'<\s*system\s*>',
    r'\[\s*INST\s*\]',
    r'</?\s*prompt\s*>',
    r'new\s+instructions?\s*:',
    r'override\s+system',
    r'you\s+are\s+now\s+a',
]

# SECURITY: Maximum input lengths to prevent abuse
MAX_QUERY_LENGTH = 2000
MAX_CONTENT_LENGTH = 10000
MAX_CONTEXT_LENGTH = 3000


@register_agent('research')
class ResearchAgent(BaseAgent):
    """Agent specialized for research and information gathering.

    Capabilities:
    - Analyze and summarize content
    - Extract key information
    - Answer questions based on context
    - Compare and contrast topics
    """

    SYSTEM_PROMPT = """You are a Research Agent specialized in information gathering, analysis, and synthesis.

Your capabilities:
1. ANALYZE: Break down complex topics into key components
2. SUMMARIZE: Create concise summaries of lengthy content
3. EXTRACT: Identify and extract key facts, dates, names, and concepts
4. COMPARE: Compare and contrast different topics or perspectives
5. ANSWER: Answer questions based on provided context

Guidelines:
- Be thorough but concise
- Cite sources when available
- Acknowledge uncertainty when information is incomplete
- Structure responses with clear sections when appropriate
- Use bullet points for lists of findings

Output Format:
- Start with a brief overview
- Present findings in organized sections
- End with key takeaways or conclusions
"""

    @property
    def agent_type(self) -> AgentType:
        """Return research agent type."""
        return AgentType.RESEARCH

    def get_system_prompt(self) -> str:
        """Return the research agent system prompt."""
        return self.SYSTEM_PROMPT

    # SECURITY: Allowed task types whitelist
    ALLOWED_TASK_TYPES = frozenset({'analyze', 'summarize', 'extract', 'compare', 'answer'})

    def _sanitize_input(self, text: str, max_length: int, field_name: str) -> str:
        """SECURITY: Sanitize user input to prevent prompt injection.

        - Truncates to max_length
        - Detects suspicious patterns
        - Removes control characters
        """
        if not isinstance(text, str):
            return ''

        # Truncate to max length
        text = text[:max_length]

        # Remove control characters (except newlines and tabs)
        text = ''.join(c for c in text if c.isprintable() or c in '\n\t')

        # Check for prompt injection patterns
        for pattern in PROMPT_INJECTION_PATTERNS:
            if re.search(pattern, text, re.IGNORECASE):
                logger.warning(
                    f"SECURITY: Potential prompt injection detected in {field_name}"
                )
                # Remove the suspicious pattern
                text = re.sub(pattern, '[REDACTED]', text, flags=re.IGNORECASE)

        return text

    def execute(self, task: AgentTask) -> AgentTask:
        """Execute a research task.

        Supported input_data keys:
        - query: The research question or topic
        - content: Content to analyze (optional)
        - task_type: 'analyze', 'summarize', 'extract', 'compare', 'answer'
        - context: Additional context (optional)

        Returns:
            Updated task with research results
        """
        task.mark_running()

        try:
            input_data = task.input_data

            # SECURITY: Validate and sanitize all user inputs
            query = self._sanitize_input(
                input_data.get('query', ''),
                MAX_QUERY_LENGTH,
                'query'
            )

            task_type = input_data.get('task_type', 'analyze')
            # SECURITY: Validate task_type against whitelist
            if task_type not in self.ALLOWED_TASK_TYPES:
                task.mark_failed(f"Invalid task_type. Must be one of: {', '.join(sorted(self.ALLOWED_TASK_TYPES))}")
                return task

            content = self._sanitize_input(
                input_data.get('content', ''),
                MAX_CONTENT_LENGTH,
                'content'
            )

            context = self._sanitize_input(
                input_data.get('context', ''),
                MAX_CONTEXT_LENGTH,
                'context'
            )

            previous_output = input_data.get('previous_output', {})
            # SECURITY: Sanitize previous output summary if present
            if isinstance(previous_output, dict) and 'summary' in previous_output:
                previous_output = {
                    **previous_output,
                    'summary': self._sanitize_input(
                        str(previous_output.get('summary', ''))[:500],
                        500,
                        'previous_output.summary'
                    )
                }

            # Build the research prompt
            prompt = self._build_prompt(query, content, task_type, context, previous_output)

            # Add step for prompt building
            self._add_step(
                action='build_prompt',
                input_data={'task_type': task_type, 'query': query[:100]},
                reasoning=f"Building {task_type} prompt for research task"
            )

            # Call LLM
            response = self._call_llm(
                prompt,
                max_tokens=self.config.get('max_tokens', 2000),
                temperature=self.config.get('temperature', 0.3)
            )

            # Add step for LLM call
            self._add_step(
                action='llm_call',
                input_data={'prompt_length': len(prompt)},
                output_data={'response_length': len(response.content)},
                reasoning="Executing research with LLM"
            )

            # Parse and structure the response
            result = self._parse_response(response.content, task_type)

            # Add execution trace to result
            result['execution_trace'] = self.get_execution_trace()
            result['usage'] = response.usage

            task.mark_completed(result)

        except Exception as e:
            # SECURITY: Log full error internally, return generic message to user
            logger.error(f"Research agent execution failed: {e}", exc_info=True)
            task.mark_failed("Research task failed. Please try again.")

        return task

    def _build_prompt(
        self,
        query: str,
        content: str,
        task_type: str,
        context: str,
        previous_output: Dict[str, Any]
    ) -> str:
        """Build the research prompt based on task type."""
        prompts = {
            'analyze': f"""Analyze the following topic/content and provide a detailed breakdown.

Topic/Query: {query}

Content to Analyze:
{content[:5000] if content else 'No specific content provided. Analyze based on the query.'}

{f'Additional Context: {context}' if context else ''}
{f'Previous Research: {previous_output.get("summary", "")}' if previous_output else ''}

Provide:
1. Overview of the topic
2. Key components and their relationships
3. Important insights
4. Potential implications or applications
""",
            'summarize': f"""Summarize the following content concisely while preserving key information.

Content to Summarize:
{content[:8000] if content else query}

{f'Focus on: {context}' if context else ''}

Provide:
1. Brief executive summary (2-3 sentences)
2. Key points (bullet list)
3. Main conclusions
""",
            'extract': f"""Extract key information from the following content.

Content:
{content[:6000] if content else query}

{f'Focus on extracting: {context}' if context else 'Extract: names, dates, facts, concepts, and statistics'}

Format the extracted information clearly with categories.
""",
            'compare': f"""Compare and contrast the following topics.

Topics: {query}

{f'Content for reference: {content[:4000]}' if content else ''}
{f'Focus on: {context}' if context else ''}

Provide:
1. Similarities
2. Differences
3. Strengths and weaknesses of each
4. Recommendation or conclusion
""",
            'answer': f"""Answer the following question based on the provided context.

Question: {query}

Context:
{content[:6000] if content else 'No specific context provided.'}

{f'Additional information: {context}' if context else ''}

Provide a clear, well-reasoned answer with supporting evidence from the context.
"""
        }

        return prompts.get(task_type, prompts['analyze'])

    def _parse_response(self, response: str, task_type: str) -> Dict[str, Any]:
        """Parse LLM response into structured result."""
        result = {
            'task_type': task_type,
            'content': response,
            'summary': self._extract_summary(response),
            'key_points': self._extract_key_points(response),
            'word_count': len(response.split())
        }

        return result

    def _extract_summary(self, response: str) -> str:
        """Extract a brief summary from the response."""
        lines = response.strip().split('\n')
        # Return first non-empty paragraph
        for line in lines:
            line = line.strip()
            if line and not line.startswith('#') and len(line) > 20:
                return line[:300] + ('...' if len(line) > 300 else '')
        return response[:300]

    def _extract_key_points(self, response: str) -> list:
        """Extract bullet points from the response."""
        points = []
        lines = response.split('\n')
        for line in lines:
            line = line.strip()
            # Match bullet points and numbered lists
            if re.match(r'^[-*•]\s+', line) or re.match(r'^\d+[.)]\s+', line):
                # Clean the point
                clean = re.sub(r'^[-*•\d.)]+\s*', '', line)
                if clean:
                    points.append(clean)
        return points[:10]  # Return top 10 points
