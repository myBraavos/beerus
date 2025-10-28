use crate::{
    client::State,
    storage::{
        storage_trait::{StorageError, StorageProviderTrait},
        utils::parse_state_row,
    },
};
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

pub struct SqlStorageProvider {
    pool: Pool<Postgres>,
}

impl SqlStorageProvider {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = Pool::<Postgres>::connect(database_url).await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl StorageProviderTrait for SqlStorageProvider {
    async fn read_state(
        &self,
        block_number: u64,
    ) -> Result<State, StorageError> {
        let query = "SELECT block_number, block_hash, root FROM state WHERE block_number = $1";
        let row: Option<(i64, String, String)> = sqlx::query_as(query)
            .bind(block_number as i64)
            .fetch_optional(&self.pool)
            .await?;

        parse_state_row(row)
    }

    async fn read_latest_state(&self) -> Result<State, StorageError> {
        let query = "SELECT block_number, block_hash, root FROM state ORDER BY block_number DESC LIMIT 1";
        let row: Option<(i64, String, String)> =
            sqlx::query_as(query).fetch_optional(&self.pool).await?;

        parse_state_row(row)
    }

    async fn write_state(&self, state: &State) -> Result<(), StorageError> {
        let query = "INSERT INTO state (block_number, block_hash, root)
                VALUES ($1, $2, $3)
                ON CONFLICT (block_number)
                DO UPDATE SET block_hash = $2, root = $3";
        sqlx::query(query)
            .bind(state.block_number)
            .bind(state.block_hash.as_ref())
            .bind(state.root.as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
