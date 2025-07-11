---
title: "Upload Test Document"
author: "Claude Code"
tags: ["test", "upload", "markdown"]
public: true
date: "2025-07-11"
---

# Upload Test Document

This is a test document to verify the file upload functionality.

## Features

- **File Upload**: Upload .md files with drag & drop or click to select
- **Metadata Parsing**: Extract frontmatter metadata from uploaded files
- **Auto-tagging**: Automatically parse tags from metadata
- **Validation**: File size and type validation

## Content

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.

### Code Example

```python
def upload_file(file):
    if file.endswith('.md'):
        return process_markdown(file)
    return None
```

### List Example

1. First item
2. Second item
3. Third item

- Bullet point 1
- Bullet point 2
- Bullet point 3

## Conclusion

This document should be successfully uploaded and parsed with all metadata extracted.