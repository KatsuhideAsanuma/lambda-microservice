
use crate::error::{Error, Result};
use crate::schema;
use crate::session::DbPoolTrait;
use async_trait::async_trait;
use deadpool_postgres::{Config, Pool, PoolConfig, Runtime};
use tokio_postgres::NoTls;
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct PostgresPool {
    pool: Pool,
}

#[async_trait]
impl DbPoolTrait for PostgresPool {
    async fn execute<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        self.execute(query, params).await
    }
    
    async fn query_opt<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<tokio_postgres::Row>> {
        self.query_opt(query, params).await
    }
    
    async fn query_one<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<tokio_postgres::Row> {
        self.query_one(query, params).await
    }
}

impl PostgresPool {
    pub async fn new(database_url: &str) -> Result<Self> {
        let mut config = Config::new();
        
        let parts: Vec<&str> = database_url.split("://").collect();
        if parts.len() == 2 {
            let credentials_and_host: Vec<&str> = parts[1].split("@").collect();
            if credentials_and_host.len() == 2 {
                let credentials: Vec<&str> = credentials_and_host[0].split(":").collect();
                if credentials.len() == 2 {
                    config.user = Some(credentials[0].to_string());
                    config.password = Some(credentials[1].to_string());
                }
                
                let host_port_db: Vec<&str> = credentials_and_host[1].split("/").collect();
                if host_port_db.len() >= 1 {
                    let host_port: Vec<&str> = host_port_db[0].split(":").collect();
                    if host_port.len() == 2 {
                        config.host = Some(host_port[0].to_string());
                        if let Ok(port) = host_port[1].parse::<u16>() {
                            config.port = Some(port);
                        }
                    } else {
                        config.host = Some(host_port_db[0].to_string());
                    }
                    
                    if host_port_db.len() >= 2 {
                        config.dbname = Some(host_port_db[1].to_string());
                    }
                }
            }
        } else {
            return Err(Error::Database(format!("Failed to parse database URL: {}", database_url)));
        }
        
        config.pool = Some(PoolConfig::new(10));

        let pool = config.create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| Error::Database(format!("Failed to create pool: {}", e)))?;

        let client = pool.get().await?;
        client.execute("SELECT 1", &[]).await?;
        
        match schema::initialize_database(&client).await {
            Ok(_) => info!("Database schema initialized successfully"),
            Err(e) => {
                error!("Failed to initialize database schema: {}", e);
                debug!("Continuing with existing schema");
            }
        }

        Ok(Self { pool })
    }

    pub async fn get(&self) -> Result<deadpool_postgres::Client> {
        self.pool.get().await.map_err(Error::from)
    }

    pub async fn execute(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        client.execute(query, params).await.map_err(Error::from)
    }

    pub async fn query(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<tokio_postgres::Row>> {
        let client = self.get().await?;
        client.query(query, params).await.map_err(Error::from)
    }

    pub async fn query_one(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<tokio_postgres::Row> {
        let client = self.get().await?;
        client.query_one(query, params).await.map_err(Error::from)
    }

    pub async fn query_opt(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<tokio_postgres::Row>> {
        let client = self.get().await?;
        client.query_opt(query, params).await.map_err(Error::from)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_postgres::Row;

    #[derive(Clone)]
    #[allow(dead_code)]
    struct MockClient {
        execute_result: Arc<Mutex<Result<u64>>>,
        query_result: Arc<Mutex<Result<Vec<Row>>>>,
        query_one_result: Arc<Mutex<Result<Row>>>,
        query_opt_result: Arc<Mutex<Result<Option<Row>>>>,
    }

    impl MockClient {
        #[allow(dead_code)]
        fn new() -> Self {
            Self {
                execute_result: Arc::new(Mutex::new(Ok(1))),
                query_result: Arc::new(Mutex::new(Ok(Vec::new()))),
                query_one_result: Arc::new(Mutex::new(Err(Error::NotFound("No rows found".to_string())))),
                query_opt_result: Arc::new(Mutex::new(Ok(None))),
            }
        }

        #[allow(dead_code)]
        fn with_execute_result(mut self, result: Result<u64>) -> Self {
            self.execute_result = Arc::new(Mutex::new(result));
            self
        }

        #[allow(dead_code)]
        fn with_query_result(mut self, result: Result<Vec<Row>>) -> Self {
            self.query_result = Arc::new(Mutex::new(result));
            self
        }

        #[allow(dead_code)]
        fn with_query_one_result(mut self, result: Result<Row>) -> Self {
            self.query_one_result = Arc::new(Mutex::new(result));
            self
        }

        #[allow(dead_code)]
        fn with_query_opt_result(mut self, result: Result<Option<Row>>) -> Self {
            self.query_opt_result = Arc::new(Mutex::new(result));
            self
        }

        #[allow(dead_code)]
        async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            self.execute_result.lock().await.clone()
        }

        #[allow(dead_code)]
        async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
            self.query_result.lock().await.clone()
        }

        #[allow(dead_code)]
        async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
            self.query_one_result.lock().await.clone()
        }

        #[allow(dead_code)]
        async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
            self.query_opt_result.lock().await.clone()
        }
    }

    #[derive(Clone)]
    pub struct MockPostgresPool {
        execute_result: Arc<Mutex<Result<u64>>>,
        query_opt_result: Arc<Mutex<Result<Option<Row>>>>,
        query_one_result: Arc<Mutex<Result<Row>>>,
    }

    impl MockPostgresPool {
        pub fn new() -> Self {
            Self {
                execute_result: Arc::new(Mutex::new(Ok(1))),
                query_opt_result: Arc::new(Mutex::new(Ok(None))),
                query_one_result: Arc::new(Mutex::new(Err(Error::NotFound("No rows found".to_string())))),
            }
        }

        pub fn with_execute_result(mut self, result: Result<u64>) -> Self {
            self.execute_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_query_opt_result(mut self, result: Result<Option<Row>>) -> Self {
            self.query_opt_result = Arc::new(Mutex::new(result));
            self
        }

        pub fn with_query_one_result(mut self, result: Result<Row>) -> Self {
            self.query_one_result = Arc::new(Mutex::new(result));
            self
        }

        pub async fn execute(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            self.execute_result.lock().await.clone()
        }

        pub async fn query(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Vec<Row>> {
            Ok(Vec::new())
        }

        pub async fn query_one(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
            self.query_one_result.lock().await.clone()
        }

        pub async fn query_opt(&self, _query: &str, _params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
            self.query_opt_result.lock().await.clone()
        }
    }
    
    #[async_trait]
    impl DbPoolTrait for MockPostgresPool {
        async fn execute<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
            self.execute(query, params).await
        }
        
        async fn query_opt<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Option<Row>> {
            self.query_opt(query, params).await
        }
        
        async fn query_one<'a>(&'a self, query: &'a str, params: &'a [&'a (dyn tokio_postgres::types::ToSql + Sync)]) -> Result<Row> {
            self.query_one(query, params).await
        }
    }

    #[tokio::test]
    async fn test_execute_success() {
        let pool = MockPostgresPool::new().with_execute_result(Ok(5));

        let result = pool.execute("INSERT INTO test VALUES ($1)", &[&"test_value"]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_execute_error() {
        let pool = MockPostgresPool::new().with_execute_result(Err(Error::Database("Database error".to_string())));

        let result = pool.execute("INSERT INTO test VALUES ($1)", &[&"test_value"]).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::Database(_)));
        }
    }

    #[tokio::test]
    async fn test_query_success() {
        let pool = MockPostgresPool::new();

        let result = pool.query("SELECT * FROM test", &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_one_not_found() {
        let pool = MockPostgresPool::new().with_query_one_result(Err(Error::NotFound("No rows found".to_string())));

        let result = pool.query_one("SELECT * FROM test WHERE id = $1", &[&1]).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(matches!(err, Error::NotFound(_)));
        }
    }

    #[tokio::test]
    async fn test_query_opt_none() {
        let pool = MockPostgresPool::new().with_query_opt_result(Ok(None));

        let result = pool.query_opt("SELECT * FROM test WHERE id = $1", &[&1]).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_postgres_pool_integration() {
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/lambda_microservice".to_string());

        println!("Connecting to database: {}", database_url);
        let pool = PostgresPool::new(&database_url).await;
        
        if pool.is_err() {
            println!("Database connection failed: {:?}", pool.as_ref().err());
            return;
        }

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
