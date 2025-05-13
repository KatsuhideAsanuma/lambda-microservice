
use crate::error::{Error, Result};
use deadpool_postgres::{Config, Pool, PoolConfig, Runtime};
use tokio_postgres::NoTls;

#[derive(Clone)]
pub struct PostgresPool {
    pool: Pool,
}

impl PostgresPool {
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut config = Config::new();
        config.url = Some(database_url.to_string());
        config.pool = Some(PoolConfig::new(10));

        let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)?;

        let client = pool.get().await?;
        client.execute("SELECT 1", &[]).await?;

        Ok(Self { pool })
    }

    pub async fn get(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.map_err(Error::from)
    }

    pub async fn execute<T>(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        client.execute(query, params).await.map_err(Error::from)
    }

    pub async fn query<T>(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<tokio_postgres::Row>> {
        let client = self.get().await?;
        client.query(query, params).await.map_err(Error::from)
    }

    pub async fn query_one<T>(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<tokio_postgres::Row> {
        let client = self.get().await?;
        client.query_one(query, params).await.map_err(Error::from)
    }

    pub async fn query_opt<T>(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<tokio_postgres::Row>> {
        let client = self.get().await?;
        client.query_opt(query, params).await.map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_postgres::Row;

    #[derive(Clone)]
    struct MockClient {
        execute_result: Arc<Mutex<Result<u64>>>,
        query_result: Arc<Mutex<Result<Vec<Row>>>>,
        query_one_result: Arc<Mutex<Result<Row>>>,
        query_opt_result: Arc<Mutex<Result<Option<Row>>>>,
    }

    impl MockClient {
        fn new() -> Self {
            Self {
                execute_result: Arc::new(Mutex::new(Ok(1))),
                query_result: Arc::new(Mutex::new(Ok(Vec::new()))),
                query_one_result: Arc::new(Mutex::new(Err(Error::NotFound("No rows found".to_string())))),
                query_opt_result: Arc::new(Mutex::new(Ok(None))),
            }
        }

        fn with_execute_result(mut self, result: Result<u64>) -> Self {
            self.execute_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_query_result(mut self, result: Result<Vec<Row>>) -> Self {
            self.query_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_query_one_result(mut self, result: Result<Row>) -> Self {
            self.query_one_result = Arc::new(Mutex::new(result));
            self
        }

        fn with_query_opt_result(mut self, result: Result<Option<Row>>) -> Self {
            self.query_opt_result = Arc::new(Mutex::new(result));
            self
        }

        async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            self.execute_result.lock().await.clone()
        }

        async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
            self.query_result.lock().await.clone()
        }

        async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
            self.query_one_result.lock().await.clone()
        }

        async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
            self.query_opt_result.lock().await.clone()
        }
    }

    struct MockPostgresPool {
        client: MockClient,
    }

    impl MockPostgresPool {
        fn new(client: MockClient) -> Self {
            Self { client }
        }

        async fn get(&self) -> Result<MockClient> {
            Ok(self.client.clone())
        }

        async fn execute(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            let client = self.get().await?;
            client.execute(query, params).await
        }

        async fn query(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
            let client = self.get().await?;
            client.query(query, params).await
        }

        async fn query_one(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
            let client = self.get().await?;
            client.query_one(query, params).await
        }

        async fn query_opt(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
            let client = self.get().await?;
            client.query_opt(query, params).await
        }
    }

    #[tokio::test]
    async fn test_execute_success() {
        let client = MockClient::new().with_execute_result(Ok(5));
        let pool = MockPostgresPool::new(client);

        let result = pool.execute("INSERT INTO test VALUES ($1)", &[&"test_value"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_execute_error() {
        let client = MockClient::new().with_execute_result(Err(Error::Database("Database error".to_string())));
        let pool = MockPostgresPool::new(client);

        let result = pool.execute("INSERT INTO test VALUES ($1)", &[&"test_value"]).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::Database(_)));
        }
    }

    #[tokio::test]
    async fn test_query_success() {
        let client = MockClient::new().with_query_result(Ok(Vec::new()));
        let pool = MockPostgresPool::new(client);

        let result = pool.query("SELECT * FROM test", &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_one_not_found() {
        let client = MockClient::new().with_query_one_result(Err(Error::NotFound("No rows found".to_string())));
        let pool = MockPostgresPool::new(client);

        let result = pool.query_one("SELECT * FROM test WHERE id = $1", &[&1]).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::NotFound(_)));
        }
    }

    #[tokio::test]
    async fn test_query_opt_none() {
        let client = MockClient::new().with_query_opt_result(Ok(None));
        let pool = MockPostgresPool::new(client);

        let result = pool.query_opt("SELECT * FROM test WHERE id = $1", &[&1]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_pool_integration() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/test".to_string());

        let pool = PostgresPool::new(&database_url).await;
        assert!(pool.is_ok());

        let pool = pool.unwrap();
        let result = pool.execute("CREATE TABLE IF NOT EXISTS test_table (id SERIAL PRIMARY KEY, name TEXT)", &[]).await;
        assert!(result.is_ok());

        let result = pool.execute("INSERT INTO test_table (name) VALUES ($1) RETURNING id", &[&"test_name"]).await;
        assert!(result.is_ok());

        let rows = pool.query("SELECT * FROM test_table WHERE name = $1", &[&"test_name"]).await;
        assert!(rows.is_ok());
        assert!(!rows.unwrap().is_empty());

        let result = pool.execute("DROP TABLE test_table", &[]).await;
        assert!(result.is_ok());
    }
}
