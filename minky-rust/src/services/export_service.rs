use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;

use crate::models::{
    ExportFormat, ExportJob, ExportRequest, ExportStatus, ExportedDocument, ImportError,
    ImportJob, ImportRequest,
};

/// Export/Import service
pub struct ExportService {
    db: PgPool,
}

impl ExportService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Start export job
    pub async fn start_export(&self, user_id: i32, request: ExportRequest) -> Result<ExportJob> {
        let job_id = uuid::Uuid::new_v4().to_string();
        let format = request.format.unwrap_or_default();

        // Count documents to export
        let doc_count = if let Some(ids) = &request.document_ids {
            ids.len() as i32
        } else if let Some(category_id) = request.category_id {
            let count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM documents WHERE category_id = $1")
                    .bind(category_id)
                    .fetch_one(&self.db)
                    .await?;
            count.0 as i32
        } else {
            let count: (i64,) =
                sqlx::query_as("SELECT COUNT(*) FROM documents WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_one(&self.db)
                    .await?;
            count.0 as i32
        };

        // TODO: Queue actual export job
        Ok(ExportJob {
            id: job_id,
            user_id,
            format,
            status: ExportStatus::Pending,
            document_count: doc_count,
            progress_percent: 0,
            download_url: None,
            error_message: None,
            created_at: Utc::now(),
            completed_at: None,
            expires_at: None,
        })
    }

    /// Get export job status
    pub async fn get_export_status(&self, job_id: &str) -> Result<Option<ExportJob>> {
        // TODO: Get from job queue/storage
        Ok(None)
    }

    /// Export documents to format
    pub async fn export_documents(
        &self,
        user_id: i32,
        request: &ExportRequest,
    ) -> Result<Vec<ExportedDocument>> {
        let mut query = String::from(
            r#"
            SELECT
                d.id,
                d.title,
                d.content,
                c.name as category_name,
                ARRAY_AGG(t.name) FILTER (WHERE t.name IS NOT NULL) as tags,
                d.created_at,
                d.updated_at,
                d.metadata
            FROM documents d
            LEFT JOIN categories c ON d.category_id = c.id
            LEFT JOIN document_tags dt ON d.id = dt.document_id
            LEFT JOIN tags t ON dt.tag_id = t.id
            WHERE d.user_id = $1
            "#,
        );

        if let Some(ids) = &request.document_ids {
            query.push_str(&format!(
                " AND d.id IN ({})",
                ids.iter()
                    .map(|id| format!("'{}'", id))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }

        if let Some(category_id) = request.category_id {
            query.push_str(&format!(" AND d.category_id = {}", category_id));
        }

        query.push_str(" GROUP BY d.id, d.title, d.content, c.name, d.created_at, d.updated_at, d.metadata ORDER BY d.created_at DESC");

        let rows: Vec<(
            uuid::Uuid,
            String,
            String,
            Option<String>,
            Option<Vec<String>>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
            Option<serde_json::Value>,
        )> = sqlx::query_as(&query)
            .bind(user_id)
            .fetch_all(&self.db)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ExportedDocument {
                id: r.0,
                title: r.1,
                content: r.2,
                category_name: r.3,
                tags: r.4.unwrap_or_default(),
                created_at: r.5,
                updated_at: r.6,
                metadata: r.7,
            })
            .collect())
    }

    /// Convert documents to JSON format
    pub fn to_json(&self, documents: &[ExportedDocument]) -> Result<String> {
        Ok(serde_json::to_string_pretty(documents)?)
    }

    /// Convert documents to CSV format
    pub fn to_csv(&self, documents: &[ExportedDocument]) -> Result<String> {
        let mut csv = String::from("id,title,content,category,tags,created_at,updated_at\n");

        for doc in documents {
            let tags = doc.tags.join(";");
            let content = doc.content.replace('"', "\"\"");
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                doc.id,
                doc.title.replace('"', "\"\""),
                content,
                doc.category_name.as_deref().unwrap_or(""),
                tags,
                doc.created_at.to_rfc3339(),
                doc.updated_at.to_rfc3339()
            ));
        }

        Ok(csv)
    }

    /// Convert documents to Markdown format
    pub fn to_markdown(&self, documents: &[ExportedDocument]) -> Result<String> {
        let mut md = String::new();

        for doc in documents {
            md.push_str(&format!("# {}\n\n", doc.title));

            if let Some(category) = &doc.category_name {
                md.push_str(&format!("**Category:** {}\n\n", category));
            }

            if !doc.tags.is_empty() {
                md.push_str(&format!("**Tags:** {}\n\n", doc.tags.join(", ")));
            }

            md.push_str(&format!("**Created:** {}\n\n", doc.created_at.to_rfc3339()));
            md.push_str("---\n\n");
            md.push_str(&doc.content);
            md.push_str("\n\n---\n\n");
        }

        Ok(md)
    }

    /// Start import job
    pub async fn start_import(
        &self,
        user_id: i32,
        _request: ImportRequest,
        content: &str,
    ) -> Result<ImportJob> {
        let job_id = uuid::Uuid::new_v4().to_string();

        // TODO: Parse content and queue import job
        let total_items = 0;

        Ok(ImportJob {
            id: job_id,
            user_id,
            status: ExportStatus::Pending,
            total_items,
            processed_items: 0,
            success_count: 0,
            error_count: 0,
            errors: vec![],
            created_at: Utc::now(),
            completed_at: None,
        })
    }

    /// Import documents from JSON
    pub async fn import_from_json(
        &self,
        user_id: i32,
        content: &str,
        category_id: Option<i32>,
    ) -> Result<ImportJob> {
        let documents: Vec<ExportedDocument> = serde_json::from_str(content)?;
        let job_id = uuid::Uuid::new_v4().to_string();
        let total = documents.len() as i32;

        let mut success_count = 0;
        let mut errors = Vec::new();

        for (idx, doc) in documents.iter().enumerate() {
            match self
                .import_single_document(user_id, doc, category_id)
                .await
            {
                Ok(_) => success_count += 1,
                Err(e) => {
                    errors.push(ImportError {
                        item_index: idx as i32,
                        item_name: Some(doc.title.clone()),
                        error_message: e.to_string(),
                    });
                }
            }
        }

        Ok(ImportJob {
            id: job_id,
            user_id,
            status: ExportStatus::Completed,
            total_items: total,
            processed_items: total,
            success_count,
            error_count: errors.len() as i32,
            errors,
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        })
    }

    /// Import single document
    async fn import_single_document(
        &self,
        user_id: i32,
        doc: &ExportedDocument,
        category_id: Option<i32>,
    ) -> Result<uuid::Uuid> {
        let id = uuid::Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO documents (id, title, content, user_id, category_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            "#,
        )
        .bind(id)
        .bind(&doc.title)
        .bind(&doc.content)
        .bind(user_id)
        .bind(category_id)
        .execute(&self.db)
        .await?;

        Ok(id)
    }
}
