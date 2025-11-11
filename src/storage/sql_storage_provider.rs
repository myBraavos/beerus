use crate::{
    client::{l1_range::L1Range, State},
    gen::Felt,
    storage::{
        storage_trait::StorageProviderTrait,
        utils::{parse_l1_range_row, parse_state_row},
    },
};
use async_trait::async_trait;
use eyre::Result;
use sqlx::{Pool, Postgres, QueryBuilder};
use std::collections::HashMap;

const INSERT_STATE_QUERY: &str =
    "INSERT INTO state (block_number, block_hash, root)
    VALUES ($1, $2, $3)
    ON CONFLICT (block_number)
    DO UPDATE SET block_hash = $2, root = $3";

#[derive(Clone)]
pub struct SqlStorageProvider {
    pool: Pool<Postgres>,
}

impl SqlStorageProvider {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = Pool::<Postgres>::connect(database_url).await?;
        let provider = Self { pool };
        if !provider.table_exists("state").await? {
            provider.create_tables().await?;
            provider.fill_default_state_data().await?;
            provider.fill_default_l1_range_data().await?;
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

    async fn create_tables(&self) -> Result<()> {
        let create_state_table_query = "CREATE TABLE IF NOT EXISTS state (
            block_number BIGINT PRIMARY KEY,
            block_hash VARCHAR(66) NOT NULL UNIQUE,
            root VARCHAR(66) NOT NULL
        )";
        sqlx::query(create_state_table_query).execute(&self.pool).await?;

        let create_l1_range_table_query =
            "CREATE TABLE IF NOT EXISTS l1_range (
            l1_start BIGINT PRIMARY KEY,
            l1_end BIGINT NOT NULL,
            l2_start BIGINT NOT NULL,
            l2_end BIGINT NOT NULL
        )";
        sqlx::query(create_l1_range_table_query).execute(&self.pool).await?;

        Ok(())
    }

    async fn fill_default_state_data(&self) -> Result<()> {
        // TODO: should be common code for all providers
        let default_data: Vec<(i64, &str, &str)> = vec![
            (1000000, "0x7256dde30ae68f43f3def9ce2a4433dd3de11b630d4f84336891bad8fe4127e", "0x7bd9798e3b03e6dfc12db132d48e4a0dc75202aa6a9b57bc40e3796137bd617"),
            (1000056, "0x56373a6b0d35130e0f7e9a3461b269317b1836aa66247744335d3d22067dd7f", "0x325ca7903f521b687dcd48736a0a6b32b506149c6d896603187e465ab3f1f74"),
            (1537726, "0x4a84a6981961b9b47b7bf1da94b7c1d25bebab57b09c22caf171a5aae3c1be8", "0x4013dab22b14596c1f579ecd8fae880af2be8b2a52084c75e70e592c11ceaf"),
            (2000000, "0x55bcdb9f4976886eb8e507dd527f478befda6831863760618ad50bf2e084a81", "0x3f4c29e48bcd9f5a706804ac5bd4adab9029ac5048a23fa9ce7c8df832082e1"),
            (2318292, "0x6592d1de9e8733706f2f30de1b92ccfb28c879582e153a8ecca8a880d9024b6", "0x31edbc87309c6012f8fc1795fb7527c518a8a9044b5bd0d5df7ae71a923150a"),
            (3000000, "0x1f810eb93546dc8d8ef9ed02b97d047068f16b891dfda97fce0612876ea82df", "0x35451d7ed149e89297555c6d6a65b1aa930544d78c6be66d9add0fde4ad3ef9"),
            (3262346, "0x58c4122809465bcea8719bc2e5d5acb787dce3eda8e0da9a72a749df99c578", "0x6dbc5b441772a4ac2b1a1b4602c5aeaa4d2404132c627c40ae59ba52d5c4eba"),
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

    async fn fill_default_l1_range_data(&self) -> Result<()> {
        // TODO: should be common code for all providers
        let default_data: Vec<(i64, i64, i64, i64)> = vec![
            (21451120, 22826732, 1000056, 1537726),
            (22826732, 23406093, 1537726, 2318292),
            (23406093, 23689489, 2318292, 3262346),
        ];
        for (l1_start, l1_end, l2_start, l2_end) in default_data.iter() {
            self.write_l1_range(&L1Range::new(
                *l1_start, *l1_end, *l2_start, *l2_end,
            ))
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl StorageProviderTrait for SqlStorageProvider {
    async fn read_state(&self, block_number: i64) -> Result<State> {
        let query = "SELECT block_number, block_hash, root FROM state WHERE block_number = $1";
        let row: Option<(i64, String, String)> = sqlx::query_as(query)
            .bind(block_number)
            .fetch_optional(&self.pool)
            .await?;

        parse_state_row(row)
    }

    async fn read_state_after(&self, block_number: i64) -> Result<State> {
        let query = "SELECT block_number, block_hash, root FROM state WHERE block_number > $1 ORDER BY block_number ASC LIMIT 1";
        let row: Option<(i64, String, String)> = sqlx::query_as(query)
            .bind(block_number)
            .fetch_optional(&self.pool)
            .await?;

        parse_state_row(row)
    }

    async fn read_states_by_range(
        &self,
        start_block: i64,
        end_block: i64,
    ) -> Result<Vec<State>> {
        let query = "SELECT block_number, block_hash, root FROM state WHERE block_number >= $1 AND block_number <= $2";
        let rows: Vec<(i64, String, String)> = sqlx::query_as(query)
            .bind(start_block)
            .bind(end_block)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(|row| parse_state_row(Some(row))).collect()
    }

    async fn read_state_by_hash(&self, block_hash: &Felt) -> Result<State> {
        let query = "SELECT block_number, block_hash, root FROM state WHERE block_hash = $1";
        let row: Option<(i64, String, String)> = sqlx::query_as(query)
            .bind(block_hash.as_ref())
            .fetch_optional(&self.pool)
            .await?;

        parse_state_row(row)
    }

    async fn read_latest_state(&self) -> Result<State> {
        let query = "SELECT block_number, block_hash, root FROM state ORDER BY block_number DESC LIMIT 1";
        let row: Option<(i64, String, String)> =
            sqlx::query_as(query).fetch_optional(&self.pool).await?;

        parse_state_row(row)
    }

    async fn write_state(&self, state: &State) -> Result<()> {
        sqlx::query(INSERT_STATE_QUERY)
            .bind(state.block_number)
            .bind(state.block_hash.as_ref())
            .bind(state.root.as_ref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    ///
    /// L1 range
    ///
    async fn read_l1_range(&self, block_number: i64) -> Result<L1Range> {
        let query = "SELECT l1_start, l1_end, l2_start, l2_end
            FROM l1_range
            WHERE l2_start <= $1 AND $1 <= l2_end";
        let row: Option<(i64, i64, i64, i64)> = sqlx::query_as(query)
            .bind(block_number)
            .fetch_optional(&self.pool)
            .await?;

        parse_l1_range_row(row)
    }

    async fn read_latest_l1_range(&self) -> Result<L1Range> {
        let query = "SELECT l1_start, l1_end, l2_start, l2_end FROM l1_range ORDER BY l1_end DESC LIMIT 1";
        let row: Option<(i64, i64, i64, i64)> =
            sqlx::query_as(query).fetch_optional(&self.pool).await?;

        parse_l1_range_row(row)
    }

    async fn find_big_range(
        &self,
        start_block: i64,
        range_size: i64,
    ) -> Result<L1Range> {
        let query = "SELECT l1_start, l1_end, l2_start, l2_end
                     FROM l1_range
                     WHERE l2_end >= $1 AND l2_end - l2_start >= $2
                     ORDER BY l2_start DESC
                     LIMIT 1";
        let row: Option<(i64, i64, i64, i64)> = sqlx::query_as(query)
            .bind(start_block)
            .bind(range_size)
            .fetch_optional(&self.pool)
            .await?;
        parse_l1_range_row(row)
    }

    async fn write_l1_range(&self, l1_range: &L1Range) -> Result<()> {
        self.write_l1_ranges(std::slice::from_ref(l1_range)).await
    }

    async fn write_l1_ranges(&self, l1_ranges: &[L1Range]) -> Result<()> {
        if l1_ranges.is_empty() {
            return Ok(());
        }

        // Deduplicate ranges by l1_start to avoid "cannot affect row a second time" error
        // Keep the last occurrence of each l1_start
        let mut unique_ranges: HashMap<i64, L1Range> = HashMap::new();
        for range in l1_ranges {
            unique_ranges.insert(range.l1_start, range.clone());
        }
        let deduplicated: Vec<L1Range> = unique_ranges.into_values().collect();

        if deduplicated.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO l1_range (l1_start, l1_end, l2_start, l2_end) ",
        );

        builder.push_values(&deduplicated, |mut b, range| {
            b.push_bind(range.l1_start)
                .push_bind(range.l1_end)
                .push_bind(range.l2_start)
                .push_bind(range.l2_end);
        });

        builder.push(
            " ON CONFLICT (l1_start)
              DO UPDATE SET
                l1_end = EXCLUDED.l1_end,
                l2_start = EXCLUDED.l2_start,
                l2_end = EXCLUDED.l2_end",
        );

        builder.build().execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
