"""
OCR Service for text extraction from images and PDFs
Provides optical character recognition capabilities
"""

import os
import logging
from typing import Dict, List
import io
from pathlib import Path

logger = logging.getLogger(__name__)

class OCRService:
    def __init__(self):
        self.tesseract_available = self._check_tesseract()
        self.pdf_tools_available = self._check_pdf_tools()
        
    def _check_tesseract(self) -> bool:
        """Check if Tesseract OCR is available"""
        try:
            import pytesseract
            pytesseract.get_tesseract_version()
            return True
        except (ImportError, Exception):
            logger.warning("Tesseract OCR not available. OCR functionality will be limited.")
            return False
    
    def _check_pdf_tools(self) -> bool:
        """Check if PDF processing tools are available"""
        try:
            import fitz  # PyMuPDF
            return True
        except ImportError:
            logger.warning("PyMuPDF not available. PDF OCR will be limited.")
            return False
    
    def is_available(self) -> bool:
        """Check if OCR service is available"""
        return self.tesseract_available or self._check_cloud_ocr()
    
    def _check_cloud_ocr(self) -> bool:
        """Check if cloud OCR services are available"""
        # Check for Google Cloud Vision API key
        google_creds = os.getenv('GOOGLE_APPLICATION_CREDENTIALS') or os.getenv('GOOGLE_CLOUD_OCR_KEY')
        if google_creds:
            return True
        
        # Check for Azure Computer Vision API key
        azure_key = os.getenv('AZURE_COMPUTER_VISION_KEY')
        if azure_key:
            return True
        
        return False
    
    def extract_text_from_image(self, image_data: bytes, language: str = 'eng') -> Dict:
        """
        Extract text from image using OCR
        
        Args:
            image_data: Binary image data
            language: Language code for OCR (default: 'eng')
            
        Returns:
            Dictionary with extracted text and metadata
        """
        try:
            # Try Tesseract first
            if self.tesseract_available:
                return self._extract_with_tesseract(image_data, language)
            
            # Fallback to cloud OCR
            if self._check_cloud_ocr():
                return self._extract_with_cloud_ocr(image_data, language)
            
            return {
                'success': False,
                'error': 'No OCR service available',
                'text': '',
                'confidence': 0,
                'method': 'none'
            }
            
        except Exception as e:
            logger.error(f"Error in OCR processing: {e}")
            return {
                'success': False,
                'error': str(e),
                'text': '',
                'confidence': 0,
                'method': 'error'
            }
    
    def _extract_with_tesseract(self, image_data: bytes, language: str) -> Dict:
        """Extract text using Tesseract OCR"""
        try:
            import pytesseract
            from PIL import Image
            
            # Open image from bytes
            image = Image.open(io.BytesIO(image_data))
            
            # Convert to RGB if necessary
            if image.mode != 'RGB':
                image = image.convert('RGB')
            
            # Configure Tesseract
            custom_config = r'--oem 3 --psm 6'
            
            # Extract text
            text = pytesseract.image_to_string(image, lang=language, config=custom_config)
            
            # Get confidence scores
            data = pytesseract.image_to_data(image, lang=language, config=custom_config, output_type=pytesseract.Output.DICT)
            confidences = [int(conf) for conf in data['conf'] if int(conf) > 0]
            avg_confidence = sum(confidences) / len(confidences) if confidences else 0
            
            return {
                'success': True,
                'text': text.strip(),
                'confidence': round(avg_confidence, 2),
                'method': 'tesseract',
                'language': language,
                'word_count': len(text.split()),
                'char_count': len(text)
            }
            
        except Exception as e:
            logger.error(f"Tesseract OCR error: {e}")
            return {
                'success': False,
                'error': str(e),
                'text': '',
                'confidence': 0,
                'method': 'tesseract_error'
            }
    
    def _extract_with_cloud_ocr(self, image_data: bytes, language: str) -> Dict:
        """Extract text using cloud OCR services"""
        # Try Google Cloud Vision first
        result = self._try_google_vision(image_data, language)
        if result['success']:
            return result
        
        # Try Azure Computer Vision
        result = self._try_azure_vision(image_data, language)
        if result['success']:
            return result
        
        return {
            'success': False,
            'error': 'No cloud OCR service available',
            'text': '',
            'confidence': 0,
            'method': 'cloud_unavailable'
        }
    
    def _try_google_vision(self, image_data: bytes, language: str) -> Dict:
        """Try Google Cloud Vision API"""
        try:
            from google.cloud import vision
            
            client = vision.ImageAnnotatorClient()
            image = vision.Image(content=image_data)
            
            # Configure language hints
            image_context = vision.ImageContext()
            if language != 'eng':
                image_context.language_hints = [language]
            
            response = client.text_detection(image=image, image_context=image_context)
            texts = response.text_annotations
            
            if texts:
                # First annotation contains the full text
                full_text = texts[0].description
                
                # Calculate average confidence
                confidences = []
                for text in texts[1:]:  # Skip the first one (full text)
                    if hasattr(text, 'confidence'):
                        confidences.append(text.confidence)
                
                avg_confidence = (sum(confidences) / len(confidences) * 100) if confidences else 85
                
                return {
                    'success': True,
                    'text': full_text.strip(),
                    'confidence': round(avg_confidence, 2),
                    'method': 'google_vision',
                    'language': language,
                    'word_count': len(full_text.split()),
                    'char_count': len(full_text)
                }
            
            return {
                'success': False,
                'error': 'No text detected',
                'text': '',
                'confidence': 0,
                'method': 'google_vision'
            }
            
        except Exception as e:
            logger.error(f"Google Vision OCR error: {e}")
            return {
                'success': False,
                'error': str(e),
                'text': '',
                'confidence': 0,
                'method': 'google_vision_error'
            }
    
    def _try_azure_vision(self, image_data: bytes, language: str) -> Dict:
        """Try Azure Computer Vision API"""
        try:
            import requests
            
            subscription_key = os.getenv('AZURE_COMPUTER_VISION_KEY')
            endpoint = os.getenv('AZURE_COMPUTER_VISION_ENDPOINT')
            
            if not subscription_key or not endpoint:
                return {
                    'success': False,
                    'error': 'Azure credentials not configured',
                    'text': '',
                    'confidence': 0,
                    'method': 'azure_config_error'
                }
            
            ocr_url = endpoint + "/vision/v3.2/ocr"
            
            headers = {
                'Ocp-Apim-Subscription-Key': subscription_key,
                'Content-Type': 'application/octet-stream'
            }
            
            params = {
                'language': language if language != 'eng' else 'en',
                'detectOrientation': 'true'
            }
            
            response = requests.post(ocr_url, headers=headers, params=params, data=image_data)
            response.raise_for_status()
            
            result = response.json()
            
            # Extract text from regions
            full_text = []
            for region in result.get('regions', []):
                for line in region.get('lines', []):
                    line_text = ' '.join([word['text'] for word in line.get('words', [])])
                    full_text.append(line_text)
            
            extracted_text = '\n'.join(full_text)
            
            return {
                'success': True,
                'text': extracted_text.strip(),
                'confidence': 90,  # Azure doesn't provide word-level confidence
                'method': 'azure_vision',
                'language': language,
                'word_count': len(extracted_text.split()),
                'char_count': len(extracted_text)
            }
            
        except Exception as e:
            logger.error(f"Azure Vision OCR error: {e}")
            return {
                'success': False,
                'error': str(e),
                'text': '',
                'confidence': 0,
                'method': 'azure_vision_error'
            }
    
    def extract_text_from_pdf(self, pdf_data: bytes) -> Dict:
        """
        Extract text from PDF using OCR for image-based PDFs
        
        Args:
            pdf_data: Binary PDF data
            
        Returns:
            Dictionary with extracted text and metadata
        """
        try:
            # First try to extract text directly (for text-based PDFs)
            direct_text = self._extract_text_from_pdf_direct(pdf_data)
            if direct_text and len(direct_text.strip()) > 100:  # Has substantial text
                return {
                    'success': True,
                    'text': direct_text,
                    'confidence': 100,
                    'method': 'pdf_direct',
                    'page_count': self._get_pdf_page_count(pdf_data),
                    'word_count': len(direct_text.split()),
                    'char_count': len(direct_text)
                }
            
            # If no text or minimal text, try OCR on PDF pages
            if self.pdf_tools_available:
                return self._extract_text_from_pdf_ocr(pdf_data)
            
            return {
                'success': False,
                'error': 'PDF processing tools not available',
                'text': direct_text or '',
                'confidence': 0,
                'method': 'pdf_unavailable'
            }
            
        except Exception as e:
            logger.error(f"PDF OCR error: {e}")
            return {
                'success': False,
                'error': str(e),
                'text': '',
                'confidence': 0,
                'method': 'pdf_error'
            }
    
    def _extract_text_from_pdf_direct(self, pdf_data: bytes) -> str:
        """Extract text directly from PDF (for text-based PDFs)"""
        try:
            import fitz  # PyMuPDF
            
            doc = fitz.open(stream=pdf_data, filetype="pdf")
            text = ""
            
            for page in doc:
                text += page.get_text()
            
            doc.close()
            return text
            
        except Exception as e:
            logger.error(f"PDF direct text extraction error: {e}")
            return ""
    
    def _extract_text_from_pdf_ocr(self, pdf_data: bytes) -> Dict:
        """Extract text from PDF using OCR on each page"""
        try:
            import fitz  # PyMuPDF
            
            doc = fitz.open(stream=pdf_data, filetype="pdf")
            all_text = []
            total_confidence = 0
            processed_pages = 0
            
            for page_num in range(len(doc)):
                page = doc[page_num]
                
                # Convert page to image
                mat = fitz.Matrix(2.0, 2.0)  # Increase resolution
                pix = page.get_pixmap(matrix=mat)
                img_data = pix.tobytes("png")
                
                # Run OCR on page image
                ocr_result = self.extract_text_from_image(img_data, language='eng')
                
                if ocr_result['success'] and ocr_result['text'].strip():
                    all_text.append(f"--- Page {page_num + 1} ---\n{ocr_result['text']}")
                    total_confidence += ocr_result['confidence']
                    processed_pages += 1
            
            doc.close()
            
            full_text = '\n\n'.join(all_text)
            avg_confidence = total_confidence / processed_pages if processed_pages > 0 else 0
            
            return {
                'success': len(all_text) > 0,
                'text': full_text,
                'confidence': round(avg_confidence, 2),
                'method': 'pdf_ocr',
                'page_count': len(doc),
                'processed_pages': processed_pages,
                'word_count': len(full_text.split()),
                'char_count': len(full_text)
            }
            
        except Exception as e:
            logger.error(f"PDF OCR processing error: {e}")
            return {
                'success': False,
                'error': str(e),
                'text': '',
                'confidence': 0,
                'method': 'pdf_ocr_error'
            }
    
    def _get_pdf_page_count(self, pdf_data: bytes) -> int:
        """Get number of pages in PDF"""
        try:
            import fitz
            doc = fitz.open(stream=pdf_data, filetype="pdf")
            count = len(doc)
            doc.close()
            return count
        except Exception:
            return 0
    
    def get_supported_languages(self) -> List[str]:
        """Get list of supported OCR languages"""
        supported = ['eng']  # English is always supported
        
        if self.tesseract_available:
            try:
                import pytesseract
                langs = pytesseract.get_languages()
                supported.extend(langs)
            except Exception:
                pass
        
        # Add common languages for cloud services
        if self._check_cloud_ocr():
            cloud_langs = ['kor', 'jpn', 'chi_sim', 'chi_tra', 'fra', 'deu', 'spa', 'ita', 'por', 'rus']
            supported.extend(cloud_langs)
        
        return list(set(supported))
    
    def process_uploaded_file(self, file_data: bytes, filename: str, language: str = 'eng') -> Dict:
        """
        Process uploaded file for OCR text extraction
        
        Args:
            file_data: Binary file data
            filename: Original filename
            language: Language code for OCR
            
        Returns:
            Dictionary with extraction results
        """
        file_ext = Path(filename).suffix.lower()
        
        if file_ext == '.pdf':
            result = self.extract_text_from_pdf(file_data)
        elif file_ext in ['.png', '.jpg', '.jpeg', '.tiff', '.bmp', '.gif']:
            result = self.extract_text_from_image(file_data, language)
        else:
            return {
                'success': False,
                'error': f'Unsupported file type: {file_ext}',
                'text': '',
                'confidence': 0,
                'method': 'unsupported_format'
            }
        
        # Add filename to result
        result['filename'] = filename
        result['file_type'] = file_ext
        
        return result

# Global OCR service instance
ocr_service = OCRService()