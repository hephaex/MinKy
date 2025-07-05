from app.models.template import DocumentTemplate
from app.models.user import User
from app import db

def create_default_templates():
    """Create default templates for the system"""
    
    # Get system user (first admin user) or create a system user
    system_user = User.query.filter_by(is_admin=True).first()
    if not system_user:
        # Create a system user if no admin exists
        system_user = User(
            username='system',
            email='system@minky.local',
            password='system',
            full_name='System User'
        )
        system_user.is_admin = True
        db.session.add(system_user)
        db.session.flush()  # Get the ID
    
    templates = [
        {
            'name': 'Meeting Notes',
            'description': 'Template for recording meeting notes with agenda, attendees, and action items',
            'category': 'Business',
            'title_template': '{meeting_title} - Meeting Notes ({date})',
            'content_template': '''# {meeting_title}

**Date:** {date}
**Time:** {time}
**Location/Platform:** {location}

## Attendees
{attendees}

## Agenda
{agenda}

## Discussion Points

### Topic 1
- Key points discussed
- Decisions made

### Topic 2
- Key points discussed
- Decisions made

## Action Items
- [ ] Action item 1 - Assigned to: {assignee1}
- [ ] Action item 2 - Assigned to: {assignee2}

## Next Meeting
**Date:** {next_meeting_date}
**Topics:** {next_meeting_topics}''',
            'is_featured': True
        },
        {
            'name': 'Project Documentation',
            'description': 'Comprehensive template for project documentation including overview, requirements, and implementation details',
            'category': 'Development',
            'title_template': '{project_name} - Project Documentation',
            'content_template': '''# {project_name}

## Project Overview
{project_description}

**Project Lead:** {project_lead}
**Start Date:** {start_date}
**Expected Completion:** {end_date}

## Objectives
{objectives}

## Requirements

### Functional Requirements
{functional_requirements}

### Non-Functional Requirements
{non_functional_requirements}

## Technical Stack
{tech_stack}

## Architecture
{architecture_description}

## Implementation Plan

### Phase 1
{phase1_description}

### Phase 2
{phase2_description}

### Phase 3
{phase3_description}

## Testing Strategy
{testing_strategy}

## Deployment Plan
{deployment_plan}

## Risk Assessment
{risks}

## Resources
{resources}''',
            'is_featured': True
        },
        {
            'name': 'API Documentation',
            'description': 'Template for documenting REST API endpoints with examples',
            'category': 'Development',
            'title_template': '{api_name} API Documentation',
            'content_template': '''# {api_name} API Documentation

## Overview
{api_description}

**Base URL:** `{base_url}`
**Version:** {version}
**Authentication:** {auth_method}

## Authentication
{auth_details}

## Endpoints

### {endpoint1_name}
**Method:** `{endpoint1_method}`
**URL:** `{endpoint1_url}`
**Description:** {endpoint1_description}

#### Request
```json
{endpoint1_request_example}
```

#### Response
```json
{endpoint1_response_example}
```

### {endpoint2_name}
**Method:** `{endpoint2_method}`
**URL:** `{endpoint2_url}`
**Description:** {endpoint2_description}

#### Request
```json
{endpoint2_request_example}
```

#### Response
```json
{endpoint2_response_example}
```

## Error Codes
| Code | Description |
|------|-------------|
| 400  | Bad Request |
| 401  | Unauthorized |
| 404  | Not Found |
| 500  | Internal Server Error |

## Rate Limiting
{rate_limiting_info}

## Examples
{usage_examples}''',
            'is_featured': False
        },
        {
            'name': 'Bug Report',
            'description': 'Standard template for reporting software bugs',
            'category': 'Development',
            'title_template': 'Bug: {bug_title}',
            'content_template': '''# Bug Report: {bug_title}

## Summary
{bug_summary}

## Environment
- **OS:** {operating_system}
- **Browser/App Version:** {browser_version}
- **Device:** {device}

## Steps to Reproduce
1. {step1}
2. {step2}
3. {step3}

## Expected Behavior
{expected_behavior}

## Actual Behavior
{actual_behavior}

## Screenshots/Videos
{media_attachments}

## Additional Information
{additional_info}

## Severity
{severity_level}

## Assigned To
{assignee}''',
            'is_featured': False
        },
        {
            'name': 'Research Notes',
            'description': 'Template for organizing research findings and references',
            'category': 'Academic',
            'title_template': 'Research Notes: {research_topic}',
            'content_template': '''# Research Notes: {research_topic}

## Research Question
{research_question}

## Methodology
{methodology}

## Key Findings

### Finding 1
{finding1}

**Source:** {source1}

### Finding 2
{finding2}

**Source:** {source2}

### Finding 3
{finding3}

**Source:** {source3}

## Analysis
{analysis}

## Conclusions
{conclusions}

## References
1. {reference1}
2. {reference2}
3. {reference3}

## Next Steps
{next_steps}''',
            'is_featured': False
        },
        {
            'name': 'Product Requirements Document',
            'description': 'Template for defining product requirements and specifications',
            'category': 'Business',
            'title_template': '{product_name} - Product Requirements Document',
            'content_template': '''# {product_name} - Product Requirements Document

## Executive Summary
{executive_summary}

## Product Overview
{product_overview}

**Product Manager:** {product_manager}
**Target Release:** {target_release}

## Problem Statement
{problem_statement}

## Goals and Objectives
{goals_objectives}

## Target Audience
{target_audience}

## User Stories

### Epic 1: {epic1_name}
{epic1_description}

#### User Story 1.1
As a {user_type}, I want {functionality} so that {benefit}.

#### User Story 1.2
As a {user_type}, I want {functionality} so that {benefit}.

### Epic 2: {epic2_name}
{epic2_description}

## Requirements

### Functional Requirements
{functional_requirements}

### Non-Functional Requirements
{non_functional_requirements}

## Success Metrics
{success_metrics}

## Timeline
{timeline}

## Dependencies
{dependencies}

## Risks and Mitigation
{risks_mitigation}''',
            'is_featured': True
        },
        {
            'name': 'Tutorial/Guide',
            'description': 'Template for creating step-by-step tutorials and guides',
            'category': 'Documentation',
            'title_template': 'How to {tutorial_topic}',
            'content_template': '''# How to {tutorial_topic}

## Overview
{tutorial_overview}

**Difficulty Level:** {difficulty_level}
**Estimated Time:** {estimated_time}

## Prerequisites
{prerequisites}

## What You'll Learn
{learning_objectives}

## Step 1: {step1_title}
{step1_content}

```{code_language}
{step1_code}
```

## Step 2: {step2_title}
{step2_content}

```{code_language}
{step2_code}
```

## Step 3: {step3_title}
{step3_content}

```{code_language}
{step3_code}
```

## Troubleshooting
{troubleshooting_tips}

## Next Steps
{next_steps}

## Additional Resources
- {resource1}
- {resource2}
- {resource3}''',
            'is_featured': False
        }
    ]
    
    for template_data in templates:
        # Check if template already exists
        existing = DocumentTemplate.query.filter_by(
            name=template_data['name'],
            created_by=system_user.id
        ).first()
        
        if not existing:
            template = DocumentTemplate(
                name=template_data['name'],
                description=template_data['description'],
                category=template_data['category'],
                title_template=template_data['title_template'],
                content_template=template_data['content_template'],
                created_by=system_user.id,
                is_public=True,
                is_featured=template_data.get('is_featured', False)
            )
            db.session.add(template)
    
    db.session.commit()
    print(f"Created {len(templates)} default templates")

if __name__ == "__main__":
    from app import create_app
    app = create_app()
    with app.app_context():
        create_default_templates()