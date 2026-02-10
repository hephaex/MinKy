"""Coding Agent for code analysis, generation, and review."""
from typing import Dict, Any, Optional, List
import logging
import re

from . import register_agent
from .base_agent import BaseAgent, AgentTask, AgentType, TaskStatus

logger = logging.getLogger(__name__)


@register_agent('coding')
class CodingAgent(BaseAgent):
    """Agent specialized for code-related tasks.

    Capabilities:
    - Generate code from requirements
    - Review and analyze existing code
    - Debug and fix issues
    - Explain code behavior
    - Refactor for improvement
    """

    SYSTEM_PROMPT = """You are a Coding Agent specialized in software development tasks.

Your capabilities:
1. GENERATE: Write code from requirements and specifications
2. REVIEW: Analyze code for quality, security, and best practices
3. DEBUG: Identify and fix bugs in code
4. EXPLAIN: Explain how code works in detail
5. REFACTOR: Improve code structure and maintainability

Guidelines:
- Write clean, readable, and well-documented code
- Follow language-specific best practices and conventions
- Consider edge cases and error handling
- Prioritize security and performance
- Provide explanations for complex logic

Output Format:
- Use proper code blocks with language specification
- Include comments for complex sections
- Separate code from explanations clearly
- Provide usage examples when relevant
"""

    @property
    def agent_type(self) -> AgentType:
        """Return coding agent type."""
        return AgentType.CODING

    def get_system_prompt(self) -> str:
        """Return the coding agent system prompt."""
        return self.SYSTEM_PROMPT

    def execute(self, task: AgentTask) -> AgentTask:
        """Execute a coding task.

        Supported input_data keys:
        - code: Existing code to work with (optional)
        - task_type: 'generate', 'review', 'debug', 'explain', 'refactor'
        - requirements: Description of what to build/fix
        - language: Programming language
        - context: Additional context (dependencies, frameworks, etc.)
        - error_message: Error message for debugging (optional)

        Returns:
            Updated task with code output
        """
        task.mark_running()

        try:
            input_data = task.input_data
            code = input_data.get('code', '')
            task_type = input_data.get('task_type', 'generate')
            requirements = input_data.get('requirements', '')
            language = input_data.get('language', 'python')
            context = input_data.get('context', '')
            error_message = input_data.get('error_message', '')
            previous_output = input_data.get('previous_output', {})

            # Build the coding prompt
            prompt = self._build_prompt(
                code, task_type, requirements, language,
                context, error_message, previous_output
            )

            # Add step for prompt building
            self._add_step(
                action='build_prompt',
                input_data={'task_type': task_type, 'language': language},
                reasoning=f"Building {task_type} prompt for {language}"
            )

            # Call LLM
            response = self._call_llm(
                prompt,
                max_tokens=self.config.get('max_tokens', 3000),
                temperature=self.config.get('temperature', 0.2)  # Lower for code
            )

            # Add step for LLM call
            self._add_step(
                action='llm_call',
                input_data={'prompt_length': len(prompt)},
                output_data={'response_length': len(response.content)},
                reasoning="Executing coding task with LLM"
            )

            # Parse and structure the response
            result = self._parse_response(response.content, task_type, language)

            # Add execution trace to result
            result['execution_trace'] = self.get_execution_trace()
            result['usage'] = response.usage

            task.mark_completed(result)

        except Exception as e:
            logger.error(f"Coding agent execution failed: {e}")
            task.mark_failed(str(e))

        return task

    def _build_prompt(
        self,
        code: str,
        task_type: str,
        requirements: str,
        language: str,
        context: str,
        error_message: str,
        previous_output: Dict[str, Any]
    ) -> str:
        """Build the coding prompt based on task type."""
        base_context = f"""
Programming Language: {language}
{f'Additional Context: {context}' if context else ''}
{f'Previous Work: {previous_output.get("code", "")[:1000]}' if previous_output else ''}
"""

        prompts = {
            'generate': f"""Generate code based on the following requirements.

{base_context}

Requirements:
{requirements}

Write clean, well-documented code that:
- Follows {language} best practices
- Includes appropriate error handling
- Has clear variable and function names
- Includes usage examples
""",
            'review': f"""Review the following code for quality, security, and best practices.

{base_context}

Code to Review:
```{language}
{code}
```

Provide:
1. Overall assessment
2. Security concerns (if any)
3. Performance issues (if any)
4. Code quality issues
5. Specific recommendations for improvement
""",
            'debug': f"""Debug the following code issue.

{base_context}

Code:
```{language}
{code}
```

{f'Error Message: {error_message}' if error_message else ''}
{f'Problem Description: {requirements}' if requirements else ''}

Identify:
1. The root cause of the issue
2. Step-by-step explanation of why it fails
3. The corrected code
4. Prevention tips for similar issues
""",
            'explain': f"""Explain the following code in detail.

{base_context}

Code:
```{language}
{code}
```

{f'Focus on: {requirements}' if requirements else ''}

Provide:
1. High-level overview of what the code does
2. Line-by-line or block-by-block explanation
3. Key concepts and patterns used
4. Potential use cases
""",
            'refactor': f"""Refactor the following code for better quality and maintainability.

{base_context}

Code:
```{language}
{code}
```

{f'Focus on: {requirements}' if requirements else ''}

Provide:
1. Refactored code with improvements
2. Explanation of changes made
3. Benefits of the refactoring
"""
        }

        return prompts.get(task_type, prompts['generate'])

    def _parse_response(
        self,
        response: str,
        task_type: str,
        language: str
    ) -> Dict[str, Any]:
        """Parse LLM response into structured result."""
        result = {
            'task_type': task_type,
            'language': language,
            'content': response,
            'code_blocks': self._extract_code_blocks(response),
            'explanation': self._extract_explanation(response)
        }

        # Add specific fields based on task type
        if task_type == 'review':
            result['issues'] = self._extract_issues(response)
            result['recommendations'] = self._extract_recommendations(response)

        return result

    def _extract_code_blocks(self, response: str) -> List[Dict[str, str]]:
        """Extract code blocks from the response."""
        blocks = []
        pattern = r'```(\w*)\n(.*?)```'
        matches = re.findall(pattern, response, re.DOTALL)

        for lang, code in matches:
            blocks.append({
                'language': lang or 'text',
                'code': code.strip()
            })

        return blocks

    def _extract_explanation(self, response: str) -> str:
        """Extract explanation text (non-code) from the response."""
        # Remove code blocks
        text = re.sub(r'```.*?```', '', response, flags=re.DOTALL)
        return text.strip()

    def _extract_issues(self, response: str) -> List[str]:
        """Extract issues from a code review response."""
        issues = []
        lines = response.split('\n')

        for line in lines:
            line = line.strip()
            # Look for issue indicators
            if any(word in line.lower() for word in ['issue', 'problem', 'bug', 'vulnerability', 'concern']):
                if re.match(r'^[-*•\d.)]+\s+', line):
                    clean = re.sub(r'^[-*•\d.)]+\s*', '', line)
                    issues.append(clean)

        return issues[:10]

    def _extract_recommendations(self, response: str) -> List[str]:
        """Extract recommendations from a code review response."""
        recommendations = []
        lines = response.split('\n')

        for line in lines:
            line = line.strip()
            # Look for recommendation indicators
            if any(word in line.lower() for word in ['recommend', 'suggest', 'consider', 'should', 'could']):
                if re.match(r'^[-*•\d.)]+\s+', line):
                    clean = re.sub(r'^[-*•\d.)]+\s*', '', line)
                    recommendations.append(clean)

        return recommendations[:10]
