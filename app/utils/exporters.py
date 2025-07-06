import os
import tempfile
from datetime import datetime
import markdown
# from weasyprint import HTML, CSS  # Commented out due to system dependencies
from docx import Document as DocxDocument
from docx.shared import Inches
from docx.enum.text import WD_PARAGRAPH_ALIGNMENT
import zipfile
import json

class DocumentExporter:
    """Handle document export to various formats"""
    
    def __init__(self, document):
        self.document = document
        self.temp_dir = tempfile.mkdtemp()
    
    def cleanup(self):
        """Clean up temporary files"""
        import shutil
        try:
            shutil.rmtree(self.temp_dir)
        except Exception:
            pass
    
    def export_to_html(self, include_styles=True):
        """Export document to HTML format"""
        html_content = markdown.markdown(
            self.document.markdown_content,
            extensions=['codehilite', 'fenced_code', 'tables', 'toc']
        )
        
        if include_styles:
            css_styles = """
            <style>
                body { 
                    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                    line-height: 1.6; 
                    max-width: 800px; 
                    margin: 0 auto; 
                    padding: 20px;
                    color: #333;
                }
                h1, h2, h3, h4, h5, h6 { 
                    color: #2c3e50; 
                    margin-top: 2em;
                    margin-bottom: 1em;
                }
                h1 { border-bottom: 2px solid #3498db; padding-bottom: 10px; }
                h2 { border-bottom: 1px solid #bdc3c7; padding-bottom: 5px; }
                code { 
                    background: #f8f9fa; 
                    padding: 2px 6px; 
                    border-radius: 3px;
                    font-family: 'Monaco', 'Consolas', monospace;
                }
                pre { 
                    background: #f8f9fa; 
                    padding: 15px; 
                    border-radius: 5px; 
                    overflow-x: auto;
                    border-left: 4px solid #3498db;
                }
                blockquote { 
                    border-left: 4px solid #3498db; 
                    margin: 1.5em 0; 
                    padding-left: 20px; 
                    color: #7f8c8d;
                    font-style: italic;
                }
                table { 
                    border-collapse: collapse; 
                    width: 100%; 
                    margin: 1em 0;
                }
                th, td { 
                    border: 1px solid #bdc3c7; 
                    padding: 12px; 
                    text-align: left;
                }
                th { 
                    background: #ecf0f1; 
                    font-weight: bold;
                }
                .document-meta {
                    background: #f8f9fa;
                    padding: 15px;
                    border-radius: 5px;
                    margin-bottom: 30px;
                    border-left: 4px solid #3498db;
                }
                .export-info {
                    margin-top: 50px;
                    padding-top: 20px;
                    border-top: 1px solid #bdc3c7;
                    font-size: 0.9em;
                    color: #7f8c8d;
                }
            </style>
            """
        else:
            css_styles = ""
        
        # Document metadata
        meta_html = f"""
        <div class="document-meta">
            <h1>{self.document.title}</h1>
            {f'<p><strong>Author:</strong> {self.document.author}</p>' if self.document.author else ''}
            <p><strong>Created:</strong> {self.document.created_at.strftime('%B %d, %Y at %I:%M %p')}</p>
            <p><strong>Last Updated:</strong> {self.document.updated_at.strftime('%B %d, %Y at %I:%M %p')}</p>
            {f'<p><strong>Tags:</strong> {", ".join(self.document.get_tag_names())}</p>' if self.document.get_tag_names() else ''}
        </div>
        """
        
        export_info = f"""
        <div class="export-info">
            <p>Exported from Minky Document Management System on {datetime.utcnow().strftime('%B %d, %Y at %I:%M %p UTC')}</p>
        </div>
        """
        
        full_html = f"""
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>{self.document.title}</title>
            {css_styles}
        </head>
        <body>
            {meta_html}
            {html_content}
            {export_info}
        </body>
        </html>
        """
        
        return full_html
    
    def export_to_pdf(self):
        """Export document to PDF format using WeasyPrint"""
        raise NotImplementedError("PDF export is currently disabled due to WeasyPrint dependencies")
        # html_content = self.export_to_html(include_styles=True)
        
        # Additional CSS for PDF
        # pdf_css = CSS(string="""
        #     @page {
        #         margin: 1in;
        #         @bottom-center {
        #             content: counter(page) " of " counter(pages);
        #             font-size: 10px;
        #             color: #666;
        #         }
        #     }
        #     body { font-size: 12px; }
        #     h1 { page-break-before: avoid; }
        #     pre, table { page-break-inside: avoid; }
        # """)
        
        # pdf_file = os.path.join(self.temp_dir, f"{self.document.id}_{self.document.title[:50]}.pdf")
        # HTML(string=html_content).write_pdf(pdf_file, stylesheets=[pdf_css])
        
        # return pdf_file
    
    def export_to_docx(self):
        """Export document to DOCX format"""
        doc = DocxDocument()
        
        # Add title
        title = doc.add_heading(self.document.title, 0)
        title.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER
        
        # Add metadata
        meta_table = doc.add_table(rows=0, cols=2)
        meta_table.style = 'Table Grid'
        
        if self.document.author:
            row = meta_table.add_row()
            row.cells[0].text = 'Author'
            row.cells[1].text = self.document.author
        
        row = meta_table.add_row()
        row.cells[0].text = 'Created'
        row.cells[1].text = self.document.created_at.strftime('%B %d, %Y at %I:%M %p')
        
        row = meta_table.add_row()
        row.cells[0].text = 'Last Updated'
        row.cells[1].text = self.document.updated_at.strftime('%B %d, %Y at %I:%M %p')
        
        if self.document.get_tag_names():
            row = meta_table.add_row()
            row.cells[0].text = 'Tags'
            row.cells[1].text = ', '.join(self.document.get_tag_names())
        
        doc.add_page_break()
        
        # Convert markdown to paragraphs (simplified)
        lines = self.document.markdown_content.split('\n')
        current_paragraph = ""
        
        for line in lines:
            line = line.strip()
            
            if line.startswith('# '):
                if current_paragraph:
                    doc.add_paragraph(current_paragraph)
                    current_paragraph = ""
                doc.add_heading(line[2:], level=1)
            elif line.startswith('## '):
                if current_paragraph:
                    doc.add_paragraph(current_paragraph)
                    current_paragraph = ""
                doc.add_heading(line[3:], level=2)
            elif line.startswith('### '):
                if current_paragraph:
                    doc.add_paragraph(current_paragraph)
                    current_paragraph = ""
                doc.add_heading(line[4:], level=3)
            elif line.startswith('```'):
                # Handle code blocks (simplified)
                if current_paragraph:
                    doc.add_paragraph(current_paragraph)
                    current_paragraph = ""
                continue
            elif line == '':
                if current_paragraph:
                    doc.add_paragraph(current_paragraph)
                    current_paragraph = ""
            else:
                if current_paragraph:
                    current_paragraph += " " + line
                else:
                    current_paragraph = line
        
        if current_paragraph:
            doc.add_paragraph(current_paragraph)
        
        # Add footer
        doc.add_page_break()
        footer_para = doc.add_paragraph()
        footer_para.add_run(f"Exported from Minky on {datetime.utcnow().strftime('%B %d, %Y')}")
        footer_para.alignment = WD_PARAGRAPH_ALIGNMENT.CENTER
        
        docx_file = os.path.join(self.temp_dir, f"{self.document.id}_{self.document.title[:50]}.docx")
        doc.save(docx_file)
        
        return docx_file
    
    def export_to_markdown(self):
        """Export document as clean markdown with metadata"""
        metadata = f"""---
title: {self.document.title}
author: {self.document.author or 'Unknown'}
created: {self.document.created_at.isoformat()}
updated: {self.document.updated_at.isoformat()}
tags: [{', '.join(f'"{tag}"' for tag in self.document.get_tag_names())}]
exported: {datetime.utcnow().isoformat()}
---

"""
        
        content = metadata + self.document.markdown_content
        
        md_file = os.path.join(self.temp_dir, f"{self.document.id}_{self.document.title[:50]}.md")
        with open(md_file, 'w', encoding='utf-8') as f:
            f.write(content)
        
        return md_file
    
    def export_to_json(self):
        """Export document as JSON with full metadata"""
        data = {
            'document': self.document.to_dict(),
            'export_info': {
                'exported_at': datetime.utcnow().isoformat(),
                'format': 'json',
                'version': '1.0'
            }
        }
        
        json_file = os.path.join(self.temp_dir, f"{self.document.id}_{self.document.title[:50]}.json")
        with open(json_file, 'w', encoding='utf-8') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)
        
        return json_file
    
    def export_bundle(self, formats=['html', 'pdf', 'docx', 'markdown', 'json']):
        """Export document in multiple formats as a ZIP bundle"""
        zip_file = os.path.join(self.temp_dir, f"{self.document.id}_{self.document.title[:50]}_bundle.zip")
        
        with zipfile.ZipFile(zip_file, 'w', zipfile.ZIP_DEFLATED) as zf:
            for format_type in formats:
                try:
                    if format_type == 'html':
                        content = self.export_to_html()
                        zf.writestr(f"{self.document.title[:50]}.html", content.encode('utf-8'))
                    elif format_type == 'pdf':
                        pdf_path = self.export_to_pdf()
                        zf.write(pdf_path, f"{self.document.title[:50]}.pdf")
                    elif format_type == 'docx':
                        docx_path = self.export_to_docx()
                        zf.write(docx_path, f"{self.document.title[:50]}.docx")
                    elif format_type == 'markdown':
                        md_path = self.export_to_markdown()
                        zf.write(md_path, f"{self.document.title[:50]}.md")
                    elif format_type == 'json':
                        json_path = self.export_to_json()
                        zf.write(json_path, f"{self.document.title[:50]}.json")
                except Exception as e:
                    # Add error log to zip
                    zf.writestr(f"export_errors_{format_type}.txt", str(e))
        
        return zip_file

def export_document(document, format_type='html', **options):
    """Convenience function to export a document"""
    exporter = DocumentExporter(document)
    
    try:
        if format_type == 'html':
            return exporter.export_to_html(**options)
        elif format_type == 'pdf':
            return exporter.export_to_pdf()
        elif format_type == 'docx':
            return exporter.export_to_docx()
        elif format_type == 'markdown':
            return exporter.export_to_markdown()
        elif format_type == 'json':
            return exporter.export_to_json()
        elif format_type == 'bundle':
            return exporter.export_bundle(**options)
        else:
            raise ValueError(f"Unsupported format: {format_type}")
    finally:
        if format_type in ['html', 'json']:
            # For string returns, cleanup immediately
            exporter.cleanup()
    
    # For file returns, cleanup is handled by the route