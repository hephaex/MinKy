#!/usr/bin/env python3
"""Test script for image processing functionality"""

import sys
import os
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from app.utils.obsidian_parser import ObsidianParser

# Test markdown content with images
test_content = """---
title: Test Document
author: Test Author
tags: [test, markdown]
---

# Test Document

This is a test document with images.

[![Image.png](https://qiita-image-store.s3.ap-northeast-1.amazonaws.com/0/215128/3555c4fe-2227-4908-b163-d32c6b4f3165.png)](https://qiita-user-contents.imgix.net/https%3A%2F%2Fqiita-image-store.s3.ap-northeast-1.amazonaws.com%2F0%2F215128%2F3555c4fe-2227-4908-b163-d32c6b4f3165.png?ixlib=rb-4.0.0&auto=format&gif-q=60&q=75&s=4b485776f62c7d7abd214b46fb33f0a4)

Regular image: ![Another image](https://example.com/image.jpg)

Local image (should not be processed): ![Local image](./local/image.png)

#test #markdown
"""

def test_image_processing():
    print("Testing image processing...")
    
    # Create test backup directory
    test_backup_dir = "test_backup"
    os.makedirs(test_backup_dir, exist_ok=True)
    
    # Create parser and process content
    parser = ObsidianParser()
    result = parser.parse_markdown(test_content, backup_dir=test_backup_dir)
    
    print("Original content:")
    print(test_content)
    print("\n" + "="*50 + "\n")
    
    print("Processed content:")
    print(result['clean_content'])
    print("\n" + "="*50 + "\n")
    
    print("Frontmatter:")
    print(result['frontmatter'])
    print("\n" + "="*50 + "\n")
    
    print("Internal links:")
    print(result['internal_links'])
    print("\n" + "="*50 + "\n")
    
    print("Hashtags:")
    print(result['hashtags'])
    print("\n" + "="*50 + "\n")
    
    # Check if img directory was created
    img_dir = os.path.join(test_backup_dir, 'img')
    if os.path.exists(img_dir):
        print("Images downloaded:")
        for img_file in os.listdir(img_dir):
            print(f"  - {img_file}")
    else:
        print("No images downloaded (img directory not created)")
    
    # Cleanup
    import shutil
    if os.path.exists(test_backup_dir):
        shutil.rmtree(test_backup_dir)
    
    print("\nTest completed!")

if __name__ == "__main__":
    test_image_processing()