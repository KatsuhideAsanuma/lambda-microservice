
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
