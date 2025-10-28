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
        let query = "SELECT block_number, block_hash, root, prev_block_hash FROM state WHERE block_number = $1";
        let row: Option<(i64, String, String, String)> = sqlx::query_as(query)
            .bind(block_number as i64)
            .fetch_optional(&self.pool)
            .await?;

        parse_state_row(row)
    }

    async fn read_latest_state(&self) -> Result<State, StorageError> {
        let query = "SELECT block_number, block_hash, root, prev_block_hash FROM state ORDER BY block_number DESC LIMIT 1";
        let row: Option<(i64, String, String, String)> =
            sqlx::query_as(query).fetch_optional(&self.pool).await?;

        parse_state_row(row)
    }

    async fn write_state(&self, state: &State) -> Result<(), StorageError> {
        let query = format!(
            "INSERT INTO state (block_number, block_hash, root, prev_block_hash)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (block_number)
                DO UPDATE SET block_hash = $2, root = $3, prev_block_hash = $4"
            );
        sqlx::query(&query)
            .bind(state.block_number as i64)
            .bind(state.block_hash.as_ref())
            .bind(state.root.as_ref())
            .bind(state.prev_block_hash.as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
