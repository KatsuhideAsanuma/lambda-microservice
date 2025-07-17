use super::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;

#[derive(Clone)]
pub struct MockClient {
    execute_result: Arc<Mutex<Result<u64>>>,
    query_result: Arc<Mutex<Result<Vec<Row>>>>,
    query_one_result: Arc<Mutex<Result<Row>>>,
    query_opt_result: Arc<Mutex<Result<Option<Row>>>>,
}

impl MockClient {
    pub fn new() -> Self {
        Self {
            execute_result: Arc::new(Mutex::new(Ok(0))),
            query_result: Arc::new(Mutex::new(Ok(Vec::new()))),
            query_one_result: Arc::new(Mutex::new(Err(Error::Database(
                "No rows returned".to_string(),
            )))),
            query_opt_result: Arc::new(Mutex::new(Ok(None))),
        }
    }

    pub fn with_execute_result(mut self, result: Result<u64>) -> Self {
        self.execute_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_query_result(mut self, result: Result<Vec<Row>>) -> Self {
        self.query_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_query_one_result(mut self, result: Result<Row>) -> Self {
        self.query_one_result = Arc::new(Mutex::new(result));
        self
    }

    pub fn with_query_opt_result(mut self, result: Result<Option<Row>>) -> Self {
        self.query_opt_result = Arc::new(Mutex::new(result));
        self
    }

    pub async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        self.execute_result.lock().await.clone()
    }

    pub async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
        self.query_result.lock().await.clone()
    }

    pub async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
        self.query_one_result.lock().await.clone()
    }

    pub async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
        self.query_opt_result.lock().await.clone()
    }
}

#[derive(Clone)]
pub struct MockPostgresPool {
    client: MockClient,
}

impl MockPostgresPool {
    pub fn new() -> Self {
        Self {
            client: MockClient::new(),
        }
    }

    pub fn with_execute_result(mut self, result: Result<u64>) -> Self {
        self.client = self.client.with_execute_result(result);
        self
    }

    pub fn with_query_opt_result(mut self, result: Result<Option<Row>>) -> Self {
        self.client = self.client.with_query_opt_result(result);
        self
    }

    pub fn with_query_one_result(mut self, result: Result<Row>) -> Self {
        self.client = self.client.with_query_one_result(result);
        self
    }

    pub async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        self.client.execute(_query, _params).await
    }

    pub async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
        self.client.query(_query, _params).await
    }

    pub async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
        self.client.query_one(_query, _params).await
    }

    pub async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
        self.client.query_opt(_query, _params).await
    }
}

impl DbPoolTrait for MockPostgresPool {
    async fn execute(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        self.execute(query, params).await
    }

    async fn query_opt(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
        self.query_opt(query, params).await
    }

    async fn query_one(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
        self.query_one(query, params).await
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    
    pub struct MockRow {
        data: std::collections::HashMap<String, String>,
    }
    
    impl MockRow {
        pub fn new() -> Self {
            Self {
                data: std::collections::HashMap::new(),
            }
        }
        
        pub fn with_data(mut self, key: &str, value: &str) -> Self {
            self.data.insert(key.to_string(), value.to_string());
            self
        }
        
        pub fn get<T: std::str::FromStr>(&self, column: &str) -> T 
        where T::Err: std::fmt::Debug {
            self.data.get(column)
                .expect(&format!("Column {} not found", column))
                .parse()
                .expect(&format!("Failed to parse column {}", column))
        }
    }
}
