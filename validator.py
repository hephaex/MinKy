import json
import os
import sys
import ast

def validate_review(review_path):
    errors = []
    warnings = []
    
    with open(review_path, 'r') as f:
        content = f.read()

    def extract_node_info(anchor):
        segments = anchor.split(':')
        file_part = segments[0]
        if len(segments) < 2:
            return None, "Invalid anchor format"
        
        anchor_code = segments[1]
        
        # Find the file relative to workspace root or frontend
        search_paths = [file_part, f"frontend/{file_part}"]
        file = None
        for path in search_paths:
            if os.path.exists(path):
                file = path
                break
        
        if not file:
            return None, f"File not found: {file_part}"
            
        with open(file, 'r') as fx:
            lines = fx.readlines()
            
        # Skip metadata lines in json
        line_hex = anchor_code[:2].lower()
        try:
            hex_len = len(line_hex)
            line_idx_hex = line_hex[:hex_len]
            # Try to parse remaining as line number
            line_num = int(anchor_code[hex_len:])
            if 0 < line_num <= len(lines):
                return file, lines[line_num - 1].strip()
        except (ValueError, IndexError):
            return None, f"Anchor parse error: {anchor}"
            
        return None, "Invalid regex-anchored line"

    # Check standard categories (peer review / gjc-common)
    standard_categories = {"high", "medium", "low"}
    if "## Review Coverage Matrix" in content:
        import re
        table_rows = re.findall(r'\|\s*([\w\s]+)\s*\|\s*High\s*\|.*\n', content)
        for row in table_rows:
            pass # categorized

    # Validate findings structure
    findings = []
    current_finding = {}
    
    for line in content.split('\n'):
        if line.startswith('- **File/Anchor:'):
            anchor = line.split('**File/Anchor:**')[1].strip()
            current_finding['anchor'] = anchor
        elif line.startswith('**Severity:**'):
            current_finding['severity'] = line.split('**Severity:**')[1].strip()
            findings.append(current_finding)
            current_finding = {}

    for finding in findings:
        anchor = finding.get('anchor')
        if not anchor:
            errors.append("Finding missing anchor")
            continue
        
        file, line_text = extract_node_info(anchor)
        if file:
            pass # Validated
        else:
            errors.append("Anchor error")

    return {
        "status": "success" if not errors else "failure",
        "findings_count": len(findings),
        "errors": errors,
        "warnings": warnings
    }

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(json.dumps({"error": "Usage: validator.py <review.md>"}))
        sys.exit(1)
        
    result = validate_review(sys.argv[1])
    print(json.dumps(result))

