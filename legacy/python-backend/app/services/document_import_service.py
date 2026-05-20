"""
Document Import Service
Handles conversion of various document formats to Markdown using markitdown
"""

import os
import tempfile
import logging
import zipfile
import json
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime, timezone
from markitdown import MarkItDown
from werkzeug.datastructures import FileStorage
from app.models.document import Document
from app.models.tag import Tag
from app.services.ai_service import ai_service
from app.utils.auto_tag import generate_tags_from_content
from app import db
import re

logger = logging.getLogger(__name__)


def _log_import_operation(operation: str, user_id: int, document_id: int = None,
                          details: dict = None) -> None:
    """SECURITY: Audit log import operations for compliance."""
    log_entry = {
        'operation': operation,
        'user_id': user_id,
        'document_id': document_id,
        'timestamp': datetime.now(timezone.utc).isoformat(),
        'details': details or {}
    }
    logger.info(f"AUDIT_IMPORT: {json.dumps(log_entry)}")

# SECURITY: Magic bytes signatures for file type validation
MAGIC_SIGNATURES = {
    'docx': b'PK\x03\x04',  # ZIP-based (Office Open XML)
    'pptx': b'PK\x03\x04',
    'xlsx': b'PK\x03\x04',
    'zip': b'PK\x03\x04',
    'pdf': b'%PDF',
    'png': b'\x89PNG\r\n\x1a\n',
    'jpg': b'\xff\xd8\xff',
    'gif': b'GIF8',
}

class DocumentImportService:
    def __init__(self):
        self.markitdown = MarkItDown()
        self.ai_service = ai_service
        
        # Supported file types and their descriptions
        # Note: Images are excluded from document conversion - they should use OCR instead
        self.supported_types = {
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'Word Document',
            'application/vnd.openxmlformats-officedocument.presentationml.presentation': 'PowerPoint Presentation',
            'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'Excel Spreadsheet',
            'application/pdf': 'PDF Document',
            'text/html': 'HTML Document',
            'text/plain': 'Text File',
            'text/markdown': 'Markdown File',
            'text/csv': 'CSV File',
            'application/json': 'JSON File',
            'application/xml': 'XML File',
            'text/xml': 'XML File',
            'application/zip': 'ZIP Archive',
            'application/octet-stream': 'Binary File'
        }

    def is_supported_file(self, file: FileStorage) -> bool:
        """Check if the file type is supported for import"""
        # Exclude image files - they should use OCR instead
        if file.mimetype and file.mimetype.startswith('image/'):
            return False
        
        fname = file.filename or ''
        return (file.mimetype in self.supported_types or
                any(fname.lower().endswith(ext) for ext in
                    ['.docx', '.pptx', '.xlsx', '.pdf', '.html', '.htm', '.txt', '.md',
                     '.csv', '.json', '.xml', '.zip']))

    def get_file_type_description(self, file: FileStorage) -> str:
        """Get a human-readable description of the file type"""
        if file.mimetype in self.supported_types:
            return str(self.supported_types[file.mimetype])

        # Fallback to extension-based detection
        fname = file.filename or ''
        ext = os.path.splitext(fname.lower())[1]
        ext_mapping = {
            '.docx': 'Word Document',
            '.pptx': 'PowerPoint Presentation', 
            '.xlsx': 'Excel Spreadsheet',
            '.pdf': 'PDF Document',
            '.html': 'HTML Document',
            '.htm': 'HTML Document',
            '.txt': 'Text File',
            '.csv': 'CSV File',
            '.json': 'JSON File',
            '.xml': 'XML File',
            '.png': 'PNG Image',
            '.jpg': 'JPEG Image',
            '.jpeg': 'JPEG Image',
            '.zip': 'ZIP Archive'
        }
        return ext_mapping.get(ext, 'Unknown File Type')

    # Maximum file upload size (50MB)
    MAX_UPLOAD_SIZE = 50 * 1024 * 1024

    # SECURITY: ZIP bomb protection limits
    MAX_ZIP_DECOMPRESSED_SIZE = 100 * 1024 * 1024  # 100MB max decompressed
    MAX_ZIP_COMPRESSION_RATIO = 100  # Max 100:1 compression ratio
    MAX_ZIP_FILES = 100  # Maximum files in a ZIP archive
    MAX_NESTED_ARCHIVE_DEPTH = 0  # SECURITY: No nested archives allowed

    def _validate_magic_bytes(self, file: FileStorage) -> bool:
        """SECURITY: Validate file content matches expected magic bytes"""
        filename = (file.filename or '').lower()
        ext = os.path.splitext(filename)[1].lstrip('.')

        # Read first 16 bytes for magic signature
        file.seek(0)
        header = file.read(16)
        file.seek(0)

        if ext in MAGIC_SIGNATURES:
            expected = MAGIC_SIGNATURES[ext]
            if not header.startswith(expected):
                logger.warning(f"Magic bytes mismatch for {filename}: expected {expected.hex()}, got {header[:len(expected)].hex()}")
                return False

        return True

    def _check_zip_bomb(self, file: FileStorage) -> bool:
        """SECURITY: Check for ZIP bomb attacks and nested archives"""
        try:
            file.seek(0)
            with zipfile.ZipFile(file, 'r') as zf:
                total_uncompressed = 0
                file_count = 0

                # SECURITY: Nested archive extensions to detect
                ARCHIVE_EXTENSIONS = {'.zip', '.tar', '.tar.gz', '.tgz', '.rar', '.7z', '.gz', '.bz2'}

                for info in zf.infolist():
                    file_count += 1
                    if file_count > self.MAX_ZIP_FILES:
                        logger.warning(f"ZIP bomb detected: too many files ({file_count})")
                        return False

                    total_uncompressed += info.file_size

                    if total_uncompressed > self.MAX_ZIP_DECOMPRESSED_SIZE:
                        logger.warning(f"ZIP bomb detected: decompressed size {total_uncompressed} exceeds limit")
                        return False

                    # Check compression ratio per file
                    if info.compress_size > 0:
                        ratio = info.file_size / info.compress_size
                        if ratio > self.MAX_ZIP_COMPRESSION_RATIO:
                            logger.warning(f"ZIP bomb detected: compression ratio {ratio:.1f} exceeds limit")
                            return False

                    # SECURITY: Check for nested archives (ZIP within ZIP)
                    filename_lower = info.filename.lower()
                    if any(filename_lower.endswith(ext) for ext in ARCHIVE_EXTENSIONS):
                        logger.warning(f"Nested archive detected in ZIP: {info.filename}")
                        return False

            file.seek(0)
            return True

        except zipfile.BadZipFile:
            logger.warning("Invalid ZIP file format")
            return False
        except Exception as e:
            logger.error(f"Error checking ZIP file: {e}")
            return False

    def convert_to_markdown(self, file: FileStorage) -> Tuple[str, Dict[str, Any]]:
        """
        Convert a file to Markdown using markitdown
        Returns: (markdown_content, metadata)
        """
        if not self.is_supported_file(file):
            raise ValueError(f"Unsupported file type: {file.mimetype}")

        # Check file size before processing to prevent resource exhaustion
        file.seek(0, 2)  # Seek to end
        file_size = file.tell()
        file.seek(0)  # Reset position

        if file_size > self.MAX_UPLOAD_SIZE:
            raise ValueError(f"File too large. Maximum size is 50MB")

        # SECURITY: Validate magic bytes match file extension
        if not self._validate_magic_bytes(file):
            raise ValueError("File content does not match declared file type")

        # SECURITY: Check for ZIP bomb if it's a ZIP-based file
        filename = (file.filename or '').lower()
        if filename.endswith(('.zip', '.docx', '.pptx', '.xlsx')):
            if not self._check_zip_bomb(file):
                raise ValueError("File failed security validation")

        # Create a temporary file
        filename = file.filename or ''
        with tempfile.NamedTemporaryFile(delete=False, suffix=os.path.splitext(filename)[1]) as temp_file:
            try:
                # Save uploaded file to temporary location
                file.save(temp_file.name)
                
                # Convert using markitdown
                result = self.markitdown.convert(temp_file.name)
                
                if not result:
                    raise ValueError(f"MarkItDown failed to process file '{file.filename}' (type: {file.mimetype}). This file type may not be supported or the file may be corrupted.")
                
                if not result.text_content or result.text_content.strip() == '':
                    # Special handling for images - they might not have extractable text
                    if file.mimetype and file.mimetype.startswith('image/'):
                        raise ValueError(f"No text content found in image '{file.filename}'. For image files with text, consider using the OCR feature instead of document conversion.")
                    else:
                        raise ValueError(f"No text content could be extracted from '{file.filename}'. The file may be empty, corrupted, or in an unsupported format.")
                
                markdown_content = result.text_content
                
                # Extract metadata if available
                metadata = {
                    'original_filename': file.filename,
                    'file_type': self.get_file_type_description(file),
                    'content_length': len(markdown_content),
                }
                
                # Try to extract title from content or use filename
                title = self._extract_title_from_content(markdown_content) or \
                       os.path.splitext(filename)[0]
                
                metadata['extracted_title'] = title
                
                return markdown_content, metadata
                
            except Exception as e:
                logger.error(f"Error converting file {file.filename}: {e}")
                raise ValueError(f"Failed to convert file: {str(e)}")
            finally:
                # Clean up temporary file
                try:
                    os.unlink(temp_file.name)
                except OSError:
                    pass

    def _extract_title_from_content(self, content: str) -> Optional[str]:
        """Extract a title from the markdown content"""
        lines = content.split('\n')
        
        # Look for markdown headers
        for line in lines:
            line = line.strip()
            if line.startswith('# '):
                return line[2:].strip()
            elif line.startswith('## '):
                return line[3:].strip()
        
        # Look for the first non-empty line as potential title
        for line in lines:
            line = line.strip()
            if line and not line.startswith('#') and len(line) < 200:
                # Clean up potential title
                title = re.sub(r'[*_`\[\]()]', '', line)
                if len(title.split()) >= 2 and len(title) < 100:
                    return title[:50] + '...' if len(title) > 50 else title
        
        return None

    def generate_auto_tags(self, content: str, title: str = '') -> List[str]:
        """Generate automatic tags based on content analysis"""
        try:
            # Use existing auto-tag functionality
            auto_tags = generate_tags_from_content(content, title)
            
            # Also try AI-based tag generation if available
            if hasattr(self.ai_service, 'generate_tags'):
                try:
                    ai_tags = self.ai_service.generate_tags(content, title)
                    # Combine and deduplicate tags
                    all_tags = list(set(auto_tags + ai_tags))
                    return all_tags[:10]  # Limit to 10 tags
                except Exception as e:
                    logger.warning(f"AI tag generation failed: {e}")
            
            return auto_tags[:8]  # Limit to 8 tags if no AI
            
        except Exception as e:
            logger.error(f"Error generating auto tags: {e}")
            # Fallback to simple keyword extraction
            return self._extract_simple_keywords(content)

    def _extract_simple_keywords(self, content: str) -> List[str]:
        """Simple keyword extraction as fallback"""
        import re
        from collections import Counter
        
        # Clean content and extract words
        clean_content = re.sub(r'[^\w\s]', ' ', content.lower())
        words = clean_content.split()
        
        # Filter out common words and short words
        stop_words = {'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to', 'for', 'of', 'with', 'by', 'is', 'are', 'was', 'were', 'be', 'been', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'could', 'should', 'this', 'that', 'these', 'those'}
        filtered_words = [word for word in words if len(word) > 3 and word not in stop_words]
        
        # Get most common words
        word_freq = Counter(filtered_words)
        common_words = [word for word, count in word_freq.most_common(5) if count > 1]
        
        return common_words

    def create_document_from_import(self, markdown_content: str, metadata: Dict[str, Any],
                                  user_id: int, auto_tag: bool = True) -> Tuple[Document, List[str]]:
        """
        Create a new document from imported content
        Returns: (document, generated_tags)

        Args:
            markdown_content: The converted markdown content
            metadata: File metadata from conversion
            user_id: REQUIRED - The ID of the user creating this document (for authorization)
            auto_tag: Whether to auto-generate tags
        """
        # SECURITY: user_id is required for proper document ownership
        if user_id is None:
            raise ValueError("user_id is required for document creation")

        try:
            # Create document
            title = metadata.get('extracted_title', 'Imported Document')

            document = Document(
                title=title,
                markdown_content=markdown_content,
                user_id=user_id,  # SECURITY: Set document owner
                document_metadata={
                    'source': 'import',
                    'original_filename': metadata.get('original_filename'),
                    'file_type': metadata.get('file_type'),
                    'import_method': 'markitdown'
                }
            )
            
            db.session.add(document)
            db.session.flush()  # Get the document ID
            
            generated_tags = []
            
            # Generate and apply auto tags if requested
            if auto_tag:
                try:
                    generated_tags = self.generate_auto_tags(markdown_content, title)
                    
                    for tag_name in generated_tags:
                        # Find or create tag
                        tag = Tag.query.filter_by(name=tag_name).first()
                        if not tag:
                            tag = Tag(name=tag_name)
                            db.session.add(tag)
                            db.session.flush()
                        
                        # Associate tag with document
                        if tag not in list(document.tags):
                            document.tags.append(tag)
                            
                except Exception as e:
                    logger.error(f"Error applying auto tags: {e}")
                    # Continue without tags if there's an error
            
            db.session.commit()
            
            logger.info(f"Successfully imported document: {title} with {len(generated_tags)} auto-generated tags")
            
            return document, generated_tags
            
        except Exception as e:
            db.session.rollback()
            logger.error(f"Error creating document from import: {e}")
            raise

    def import_file(self, file: FileStorage, user_id: int, auto_tag: bool = True) -> Dict[str, Any]:
        """
        Full import process: convert file and create document
        Returns: result dictionary with document and metadata

        Args:
            file: The uploaded file
            user_id: REQUIRED - User ID for document ownership
            auto_tag: Whether to auto-generate tags
        """
        # SECURITY: user_id is required for proper authorization
        if user_id is None:
            raise ValueError("user_id is required for document import")

        try:
            # Convert file to markdown
            markdown_content, metadata = self.convert_to_markdown(file)

            # Create document with user ownership
            document, generated_tags = self.create_document_from_import(
                markdown_content, metadata, user_id=user_id, auto_tag=auto_tag
            )

            # SECURITY: Audit log successful import
            _log_import_operation(
                'import_success',
                user_id,
                document.id,
                {
                    'filename': file.filename,
                    'file_type': metadata.get('file_type'),
                    'content_length': len(markdown_content),
                    'tags_generated': len(generated_tags)
                }
            )

            return {
                'success': True,
                'document': {
                    'id': document.id,
                    'title': document.title,
                    'content_preview': document.markdown_content[:200] + '...' if len(document.markdown_content) > 200 else document.markdown_content,
                    'created_at': document.created_at.isoformat() if document.created_at else None
                },
                'metadata': metadata,
                'tags': generated_tags,
                'message': f"Successfully imported {metadata['file_type']} as '{document.title}'"
            }

        except ValueError as e:
            # ValueError is expected for validation errors - safe to return
            # SECURITY: Audit log validation failure
            _log_import_operation(
                'import_validation_failed',
                user_id,
                details={'filename': file.filename, 'error': str(e)}
            )
            logger.warning(f"Import validation failed for file {file.filename}: {e}")
            return {
                'success': False,
                'error': str(e),
                'message': f"Failed to import {file.filename}"
            }
        except Exception as e:
            # SECURITY: Audit log import failure (without detailed error)
            _log_import_operation(
                'import_failed',
                user_id,
                details={'filename': file.filename}
            )
            # SECURITY: Log detailed error but return generic message to prevent info leak
            logger.error(f"Import failed for file {file.filename}: {e}", exc_info=True)
            return {
                'success': False,
                'error': 'An unexpected error occurred during import',
                'message': f"Failed to import {file.filename}"
            }

# Singleton instance
document_import_service = DocumentImportService()