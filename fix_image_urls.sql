-- Script to fix image URLs in database documents
-- Changes https://localhost/img/ to /img/

UPDATE documents 
SET markdown_content = REPLACE(markdown_content, 'https://localhost/img/', '/img/')
WHERE markdown_content LIKE '%https://localhost/img/%';