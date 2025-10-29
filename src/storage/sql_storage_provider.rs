use crate::{
    client::State,
    storage::{
        storage_trait::{StorageError, StorageProviderTrait},
        utils::parse_state_row,
    },
};
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

const INSERT_STATE_QUERY: &str =
    "INSERT INTO state (block_number, block_hash, root)
    VALUES ($1, $2, $3)
    ON CONFLICT (block_number)
    DO UPDATE SET block_hash = $2, root = $3";

pub struct SqlStorageProvider {
    pool: Pool<Postgres>,
}

impl SqlStorageProvider {
    pub async fn new(database_url: &str) -> Result<Self, StorageError> {
        let pool = Pool::<Postgres>::connect(database_url).await?;
        let provider = Self { pool };
        if !provider.table_exists("state").await? {
            provider.create_tables().await?;
            provider.fill_default_data().await?;
        }
        Ok(provider)
    }

    async fn table_exists(
        &self,
        table_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*)
            FROM information_schema.tables
            WHERE table_schema = 'public'
              AND table_name = $1
            "#,
        )
        .bind(table_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.0 > 0)
    }

    async fn create_tables(&self) -> Result<(), StorageError> {
        let create_state_table_query = "CREATE TABLE IF NOT EXISTS state (
            block_number BIGINT PRIMARY KEY,
            block_hash VARCHAR(66) NOT NULL UNIQUE,
            root VARCHAR(66) NOT NULL
        )";
        let res =
            sqlx::query(create_state_table_query).execute(&self.pool).await?;
        tracing::info!("created state table: {:?}", res);
        Ok(())
    }

    async fn fill_default_data(&self) -> Result<(), StorageError> {
        // TODO: should be common code for all providers
        let default_data = [
            (1000000_i64, "0x7256dde30ae68f43f3def9ce2a4433dd3de11b630d4f84336891bad8fe4127e", "0x7bd9798e3b03e6dfc12db132d48e4a0dc75202aa6a9b57bc40e3796137bd617"),
            (2000000, "0x55bcdb9f4976886eb8e507dd527f478befda6831863760618ad50bf2e084a81", "0x3f4c29e48bcd9f5a706804ac5bd4adab9029ac5048a23fa9ce7c8df832082e1"),
            (3000000, "0x1f810eb93546dc8d8ef9ed02b97d047068f16b891dfda97fce0612876ea82df", "0x35451d7ed149e89297555c6d6a65b1aa930544d78c6be66d9add0fde4ad3ef9"),
        ];
        for (block_number, block_hash, root) in default_data.iter() {
            sqlx::query(INSERT_STATE_QUERY)
                .bind(block_number)
                .bind(block_hash)
                .bind(root)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
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
        sqlx::query(INSERT_STATE_QUERY)
            .bind(state.block_number)
            .bind(state.block_hash.as_ref())
            .bind(state.root.as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
