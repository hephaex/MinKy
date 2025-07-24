#!/usr/bin/env python3
"""
Script to fix image URLs in backup files
Changes https://localhost/img/ to /img/
"""

import os
import re
import glob

def fix_backup_image_urls():
    """Fix image URLs in backup files"""
    backup_dir = "/Users/mare/Simon/minky/backup"
    
    # Find all .md files in backup directory
    md_files = []
    for root, dirs, files in os.walk(backup_dir):
        for file in files:
            if file.endswith('.md'):
                md_files.append(os.path.join(root, file))
    
    print(f"Found {len(md_files)} markdown files to check")
    
    updated_count = 0
    
    for file_path in md_files:
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            # Check if the file contains the old URL pattern
            old_pattern = r'https://localhost/img/'
            new_url = '/img/'
            
            if re.search(old_pattern, content):
                # Replace the URLs
                updated_content = re.sub(old_pattern, new_url, content)
                
                # Write back to file
                with open(file_path, 'w', encoding='utf-8') as f:
                    f.write(updated_content)
                
                updated_count += 1
                print(f"Updated: {file_path}")
        
        except Exception as e:
            print(f"Error processing {file_path}: {e}")
    
    print(f"✅ Successfully updated {updated_count} files")

if __name__ == "__main__":
    fix_backup_image_urls()