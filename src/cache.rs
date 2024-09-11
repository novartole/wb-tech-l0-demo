use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, IntoConnectionInfo};
use tracing::debug;

use crate::{error::Error, model::Order, state::CacheOrder};

fn get_order_key(order_id: &str) -> String {
    format!("order{}", order_id)
}

#[derive(Clone)]
pub struct RedisCache {
    pool: Pool<RedisConnectionManager>,
}

impl RedisCache {
    pub async fn try_new(params: &str) -> Result<Self, redis::RedisError> {
        debug!(cache = "redis", "configure with params: {}", params);

        let config = params.into_connection_info()?;
        let manager = RedisConnectionManager::new(config)?;
        let pool = Pool::builder().build(manager).await?;

        Ok(Self { pool })
    }
}

impl CacheOrder for RedisCache {
    async fn get_order(&self, order_id: &str) -> Result<Option<Order>, Error> {
        let key = get_order_key(order_id);

        debug!(cache = "redis", "get order by key: {}", key);
        Ok(self.pool.get().await?.get(key).await?)
    }

    async fn insert_order(&self, order: &Order) -> Result<(), Error> {
        let key = get_order_key(&order.order_uid);
        // hardcoded, but it might be taken from config
        let secs = 60;

        debug!(
            cache = "redis",
            ?order,
            secs,
            "insert order with key: {:?}",
            key
        );
        Ok(self.pool.get().await?.set_ex(key, order, secs).await?)
    }
}
