use anyhow::Result;
use regex::Regex;
use sqlx::PgPool;

use crate::models::{
    ApplyTemplateRequest, CreateTemplate, Template, TemplatePreview, TemplateQuery,
    TemplateVariable, UpdateTemplate,
};

/// Template service
pub struct TemplateService {
    db: PgPool,
}

impl TemplateService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// List templates
    pub async fn list_templates(
        &self,
        user_id: i32,
        query: TemplateQuery,
    ) -> Result<Vec<Template>> {
        let page = query.page.unwrap_or(1);
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = (page - 1) * limit;

        let rows: Vec<(
            i32,
            String,
            Option<String>,
            String,
            Option<i32>,
            Option<String>,
            Option<serde_json::Value>,
            bool,
            i64,
            i32,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            r#"
            SELECT
                t.id,
                t.name,
                t.description,
                t.content,
                t.category_id,
                c.name as category_name,
                t.variables,
                t.is_public,
                t.usage_count,
                t.created_by,
                u.username as created_by_name,
                t.created_at,
                t.updated_at
            FROM templates t
            LEFT JOIN categories c ON t.category_id = c.id
            LEFT JOIN users u ON t.created_by = u.id
            WHERE (t.is_public = true OR t.created_by = $1)
              AND ($2::int IS NULL OR t.category_id = $2)
              AND ($3::text IS NULL OR t.name ILIKE '%' || $3 || '%')
              AND ($4::bool IS NULL OR t.is_public = $4)
            ORDER BY t.usage_count DESC, t.created_at DESC
            LIMIT $5 OFFSET $6
            "#,
        )
        .bind(user_id)
        .bind(query.category_id)
        .bind(&query.search)
        .bind(query.is_public)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let variables: Vec<TemplateVariable> = r
                    .6
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                Template {
                    id: r.0,
                    name: r.1,
                    description: r.2,
                    content: r.3,
                    category_id: r.4,
                    category_name: r.5,
                    variables,
                    tags: vec![], // TODO: Add tags support
                    is_public: r.7,
                    usage_count: r.8,
                    created_by: r.9,
                    created_by_name: r.10,
                    created_at: r.11,
                    updated_at: r.12,
                }
            })
            .collect())
    }

    /// Get template by ID
    pub async fn get_template(&self, user_id: i32, template_id: i32) -> Result<Option<Template>> {
        let row: Option<(
            i32,
            String,
            Option<String>,
            String,
            Option<i32>,
            Option<String>,
            Option<serde_json::Value>,
            bool,
            i64,
            i32,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            r#"
            SELECT
                t.id,
                t.name,
                t.description,
                t.content,
                t.category_id,
                c.name as category_name,
                t.variables,
                t.is_public,
                t.usage_count,
                t.created_by,
                u.username as created_by_name,
                t.created_at,
                t.updated_at
            FROM templates t
            LEFT JOIN categories c ON t.category_id = c.id
            LEFT JOIN users u ON t.created_by = u.id
            WHERE t.id = $1 AND (t.is_public = true OR t.created_by = $2)
            "#,
        )
        .bind(template_id)
        .bind(user_id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|r| {
            let variables: Vec<TemplateVariable> = r
                .6
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            Template {
                id: r.0,
                name: r.1,
                description: r.2,
                content: r.3,
                category_id: r.4,
                category_name: r.5,
                variables,
                tags: vec![],
                is_public: r.7,
                usage_count: r.8,
                created_by: r.9,
                created_by_name: r.10,
                created_at: r.11,
                updated_at: r.12,
            }
        }))
    }

    /// Create template
    pub async fn create_template(&self, user_id: i32, create: CreateTemplate) -> Result<Template> {
        let variables_json = create
            .variables
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;

        let row: (i32,) = sqlx::query_as(
            r#"
            INSERT INTO templates (name, description, content, category_id, variables, is_public, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            RETURNING id
            "#,
        )
        .bind(&create.name)
        .bind(&create.description)
        .bind(&create.content)
        .bind(create.category_id)
        .bind(variables_json)
        .bind(create.is_public.unwrap_or(false))
        .bind(user_id)
        .fetch_one(&self.db)
        .await?;

        self.get_template(user_id, row.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created template"))
    }

    /// Update template
    pub async fn update_template(
        &self,
        user_id: i32,
        template_id: i32,
        update: UpdateTemplate,
    ) -> Result<Template> {
        // Verify ownership
        let owner_check: Option<(i32,)> =
            sqlx::query_as("SELECT created_by FROM templates WHERE id = $1")
                .bind(template_id)
                .fetch_optional(&self.db)
                .await?;

        if let Some((owner_id,)) = owner_check {
            if owner_id != user_id {
                return Err(anyhow::anyhow!("Not authorized to update this template"));
            }
        } else {
            return Err(anyhow::anyhow!("Template not found"));
        }

        if let Some(name) = &update.name {
            sqlx::query("UPDATE templates SET name = $1, updated_at = NOW() WHERE id = $2")
                .bind(name)
                .bind(template_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(description) = &update.description {
            sqlx::query("UPDATE templates SET description = $1, updated_at = NOW() WHERE id = $2")
                .bind(description)
                .bind(template_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(content) = &update.content {
            sqlx::query("UPDATE templates SET content = $1, updated_at = NOW() WHERE id = $2")
                .bind(content)
                .bind(template_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(variables) = &update.variables {
            let json = serde_json::to_value(variables)?;
            sqlx::query("UPDATE templates SET variables = $1, updated_at = NOW() WHERE id = $2")
                .bind(json)
                .bind(template_id)
                .execute(&self.db)
                .await?;
        }

        if let Some(is_public) = update.is_public {
            sqlx::query("UPDATE templates SET is_public = $1, updated_at = NOW() WHERE id = $2")
                .bind(is_public)
                .bind(template_id)
                .execute(&self.db)
                .await?;
        }

        self.get_template(user_id, template_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Template not found"))
    }

    /// Delete template
    pub async fn delete_template(&self, user_id: i32, template_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM templates WHERE id = $1 AND created_by = $2")
            .bind(template_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Preview template with variables
    pub fn preview_template(
        &self,
        template: &Template,
        variables: Option<&serde_json::Value>,
    ) -> Result<TemplatePreview> {
        let mut content = template.content.clone();
        let mut missing_variables = Vec::new();

        // Find all {{variable}} patterns
        let re = Regex::new(r"\{\{(\w+)\}\}")?;

        for cap in re.captures_iter(&template.content) {
            let var_name = &cap[1];
            let placeholder = &cap[0];

            let value = variables
                .and_then(|v| v.get(var_name))
                .and_then(|v| v.as_str())
                .or_else(|| {
                    template
                        .variables
                        .iter()
                        .find(|v| v.name == var_name)
                        .and_then(|v| v.default_value.as_deref())
                });

            if let Some(val) = value {
                content = content.replace(placeholder, val);
            } else {
                let is_required = template
                    .variables
                    .iter()
                    .find(|v| v.name == var_name)
                    .map(|v| v.required)
                    .unwrap_or(false);

                if is_required {
                    missing_variables.push(var_name.to_string());
                }
            }
        }

        Ok(TemplatePreview {
            content,
            title: None,
            missing_variables,
        })
    }

    /// Apply template to create document
    pub async fn apply_template(
        &self,
        user_id: i32,
        request: ApplyTemplateRequest,
    ) -> Result<uuid::Uuid> {
        let template = self
            .get_template(user_id, request.template_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Template not found"))?;

        let preview = self.preview_template(&template, request.variables.as_ref())?;

        if !preview.missing_variables.is_empty() {
            return Err(anyhow::anyhow!(
                "Missing required variables: {}",
                preview.missing_variables.join(", ")
            ));
        }

        let doc_id = uuid::Uuid::new_v4();
        let title = request
            .title
            .unwrap_or_else(|| format!("From template: {}", template.name));

        sqlx::query(
            r#"
            INSERT INTO documents (id, title, content, user_id, category_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            "#,
        )
        .bind(doc_id)
        .bind(&title)
        .bind(&preview.content)
        .bind(user_id)
        .bind(request.category_id.or(template.category_id))
        .execute(&self.db)
        .await?;

        // Increment usage count
        sqlx::query("UPDATE templates SET usage_count = usage_count + 1 WHERE id = $1")
            .bind(request.template_id)
            .execute(&self.db)
            .await?;

        Ok(doc_id)
    }
}
