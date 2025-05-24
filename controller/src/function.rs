use crate::{
    api,
    error::{Error, Result},
    session::DbPoolTrait,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::Row;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: Uuid,
    pub language: String,
    pub title: String,
    pub language_title: String,
    pub description: Option<String>,
    pub schema_definition: Option<serde_json::Value>,
    pub examples: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub is_active: bool,
    pub version: String,
    pub tags: Option<Vec<String>>,
    pub script_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionQuery {
    pub language: Option<String>,
    pub user_id: Option<String>,
    pub r#type: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl Default for FunctionQuery {
    fn default() -> Self {
        Self {
            language: None,
            user_id: None,
            r#type: None,
            page: Some(1),
            per_page: Some(20),
        }
    }
}

pub struct FunctionManager<D: DbPoolTrait> {
    db_pool: D,
}

impl<D: DbPoolTrait> FunctionManager<D> {
    pub fn new(db_pool: D) -> Self {
        Self { db_pool }
    }

    pub async fn get_functions(&self, query: &FunctionQuery) -> Result<Vec<Function>> {
        let mut sql = "SELECT id, language, title, language_title, description, schema_definition, 
                      examples, created_at, updated_at, created_by, is_active, version, tags 
                      FROM meta.functions WHERE 1=1".to_string();
        
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_index = 1;
        
        if let Some(language) = &query.language {
            sql.push_str(&format!(" AND language = ${}", param_index));
            params.push(language);
            param_index += 1;
        }
        
        if let Some(user_id) = &query.user_id {
            sql.push_str(&format!(" AND created_by = ${}", param_index));
            params.push(user_id);
            param_index += 1;
        }
        
        if let Some(r#type) = &query.r#type {
            if r#type == "predefined" {
                sql.push_str(" AND created_by IS NULL");
            } else if r#type == "dynamic" {
                sql.push_str(" AND created_by IS NOT NULL");
            }
        }
        
        let page = query.page.unwrap_or(1);
        let per_page = query.per_page.unwrap_or(20);
        let offset = (page - 1) * per_page;
        
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", 
                             param_index, param_index + 1));
        params.push(&per_page);
        params.push(&offset);
        
        let rows = self.db_pool.query_opt(&sql, &params).await?
            .map(|_| Vec::new()) // This is a placeholder, we need to implement proper row handling
            .unwrap_or_default();
        
        let functions = rows.into_iter().map(|row| self.row_to_function(&row)).collect();
        
        Ok(functions)
    }
    
    fn row_to_function(&self, row: &Row) -> Function {
        Function {
            id: row.get("id"),
            language: row.get("language"),
            title: row.get("title"),
            language_title: row.get("language_title"),
            description: row.get("description"),
            schema_definition: row.get("schema_definition"),
            examples: row.get("examples"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            created_by: row.get("created_by"),
            is_active: row.get("is_active"),
            version: row.get("version"),
            tags: row.get("tags"),
            script_content: None,
        }
    }
}

#[async_trait]
impl<D: DbPoolTrait + Send + Sync> api::FunctionManagerTrait for FunctionManager<D> {
    async fn get_functions<'a>(&'a self, query: &'a FunctionQuery) -> Result<Vec<Function>> {
        self.get_functions(query).await
    }
    
    async fn get_function<'a>(&'a self, language_title: &'a str) -> Result<Option<Function>> {
        let query = "SELECT id, language, title, language_title, description, schema_definition, 
                  examples, created_at, updated_at, created_by, is_active, version, tags 
                  FROM meta.functions 
                  WHERE language_title = $1";
            
        let row_opt = self.db_pool.query_opt(query, &[&language_title]).await?;
        
        if row_opt.is_none() {
            return Ok(None);
        }
        
        let function = self.row_to_function(&row_opt.unwrap());
        Ok(Some(function))
    }
    
    async fn create_function<'a>(&'a self, function: &'a Function) -> Result<Function> {
        let query = "INSERT INTO meta.functions (
                    id, language, title, language_title, description, schema_definition,
                    examples, created_at, updated_at, created_by, is_active, version, tags
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
                ) RETURNING id";
            
        let now = Utc::now();
        let id = Uuid::new_v4();
        
        self.db_pool.execute(
            query,
            &[
                &id,
                &function.language,
                &function.title,
                &function.language_title,
                &function.description,
                &function.schema_definition,
                &function.examples,
                &now,
                &now,
                &function.created_by,
                &function.is_active,
                &function.version,
                &function.tags,
            ],
        ).await?;
        
        if let Some(script_content) = &function.script_content {
            let script_query = "INSERT INTO meta.scripts (
                        function_id, content, created_at, updated_at
                    ) VALUES (
                        $1, $2, $3, $4
                    )";
                
            self.db_pool.execute(
                script_query,
                &[&id, &script_content, &now, &now],
            ).await?;
        }
        
        let mut created_function = function.clone();
        created_function.id = id;
        created_function.created_at = now;
        created_function.updated_at = now;
        
        Ok(created_function)
    }
    
    async fn update_function<'a>(&'a self, function: &'a Function) -> Result<Function> {
        let query = "UPDATE meta.functions SET
                    language = $1,
                    title = $2,
                    language_title = $3,
                    description = $4,
                    schema_definition = $5,
                    examples = $6,
                    updated_at = $7,
                    is_active = $8,
                    version = $9,
                    tags = $10
                WHERE id = $11
                RETURNING id";
            
        let now = Utc::now();
        
        let result = self.db_pool.execute(
            query,
            &[
                &function.language,
                &function.title,
                &function.language_title,
                &function.description,
                &function.schema_definition,
                &function.examples,
                &now,
                &function.is_active,
                &function.version,
                &function.tags,
                &function.id,
            ],
        ).await?;
        
        if result == 0 {
            return Err(Error::NotFound(format!(
                "Function with id {} not found",
                function.id
            )));
        }
        
        if let Some(script_content) = &function.script_content {
            let check_query = "SELECT 1 FROM meta.scripts WHERE function_id = $1";
            
            let script_exists = self.db_pool.query_opt(check_query, &[&function.id]).await?.is_some();
            
            if script_exists {
                let script_query = "UPDATE meta.scripts SET
                            content = $1,
                            updated_at = $2
                        WHERE function_id = $3";
                    
                self.db_pool.execute(
                    script_query,
                    &[&script_content, &now, &function.id],
                ).await?;
            } else {
                let script_query = "INSERT INTO meta.scripts (
                            function_id, content, created_at, updated_at
                        ) VALUES (
                            $1, $2, $3, $4
                        )";
                    
                self.db_pool.execute(
                    script_query,
                    &[&function.id, &script_content, &now, &now],
                ).await?;
            }
        }
        
        let mut updated_function = function.clone();
        updated_function.updated_at = now;
        
        Ok(updated_function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::tests::MockPostgresPool;
    use crate::api::FunctionManagerTrait;
    use chrono::Utc;

    fn create_test_function() -> Function {
        Function {
            id: Uuid::new_v4(),
            language: "nodejs".to_string(),
            title: "calculator".to_string(),
            language_title: "nodejs-calculator".to_string(),
            description: Some("Test function".to_string()),
            schema_definition: None,
            examples: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            created_by: None,
            is_active: true,
            version: "1.0.0".to_string(),
            tags: Some(vec!["test".to_string()]),
            script_content: None,
        }
    }

    #[tokio::test]
    async fn test_get_functions() {
        let pool = MockPostgresPool::new()
            .with_query_opt_result(Ok(None));
        
        let function_manager = FunctionManager::new(pool);
        
        let query = FunctionQuery {
            language: Some("nodejs".to_string()),
            user_id: None,
            r#type: Some("predefined".to_string()),
            page: Some(1),
            per_page: Some(10),
        };
        
        let result = function_manager.get_functions(&query).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_get_function() {
        let pool = MockPostgresPool::new()
            .with_query_opt_result(Ok(None));
        
        let function_manager = FunctionManager::new(pool);
        let result = function_manager.get_function("nodejs-calculator").await;
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
    
    #[tokio::test]
    async fn test_create_function() {
        let pool = MockPostgresPool::new();
        let function_manager = FunctionManager::new(pool);
        
        let function = create_test_function();
        let result = function_manager.create_function(&function).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_update_function() {
        let pool = MockPostgresPool::new();
        
        let function_manager = FunctionManager::new(pool);
        
        let function = create_test_function();
        let result = function_manager.update_function(&function).await;
        
        assert!(result.is_ok());
    }
}
