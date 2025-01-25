use redis::{
    aio::{ConnectionLike, ConnectionManager},
    Cmd, Pipeline, RedisFuture, RedisResult, Value,
};

use crate::config::RedisConfig;

#[derive(Clone)]
pub struct RedisClient {
    manager: ConnectionManager,
    config: RedisConfig,
}

impl RedisClient {
    pub async fn new(config: RedisConfig) -> RedisResult<Self> {
        let manager = redis::Client::open(config.url.as_ref())?
            .get_connection_manager()
            .await?;
        Ok(Self { manager, config })
    }

    pub fn manager(&self) -> &ConnectionManager {
        &self.manager
    }

    pub fn config(&self) -> &RedisConfig {
        &self.config
    }
}

impl ConnectionLike for RedisClient {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        self.manager.req_packed_command(cmd)
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        self.manager.req_packed_commands(cmd, offset, count)
    }

    fn get_db(&self) -> i64 {
        self.manager.get_db()
    }
}

impl<'a> From<&'a RedisClient> for &'a RedisConfig {
    fn from(client: &'a RedisClient) -> Self {
        &client.config
    }
}
