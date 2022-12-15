use async_std::sync::{Mutex, MutexGuard};
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    MysqlConnection,
};
use std::sync::Arc;
use tide::{utils::async_trait, Middleware, Next, Request};

pub type PooledMysqlConn = PooledConnection<ConnectionManager<MysqlConnection>>;
pub type PoolMysqlConn = Pool<ConnectionManager<MysqlConnection>>;

#[derive(Clone)]
pub struct DieselMiddleware {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

impl DieselMiddleware {
    pub fn new(db_uri: &'_ str) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let manager = ConnectionManager::<MysqlConnection>::new(db_uri);
        let mysql_conn = diesel::r2d2::Builder::<ConnectionManager<MysqlConnection>>::new()
            .max_size(1)
            .build(manager)
            .map_err(|e| Box::new(e))?;
        Ok(Self { pool: mysql_conn })
    }
}

impl AsRef<Pool<ConnectionManager<MysqlConnection>>> for DieselMiddleware {
    fn as_ref(&self) -> &Pool<ConnectionManager<MysqlConnection>> {
        &self.pool
    }
}

impl From<Pool<ConnectionManager<MysqlConnection>>> for DieselMiddleware {
    fn from(pool: Pool<ConnectionManager<MysqlConnection>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl<State> Middleware<State> for DieselMiddleware
where
    State: Clone + Send + Sync + 'static,
{
    async fn handle(&self, mut req: Request<State>, next: Next<'_, State>) -> tide::Result {
        if req.ext::<Arc<Mutex<PoolMysqlConn>>>().is_some() {
            return Ok(next.run(req).await);
        }
        let conn: Arc<PoolMysqlConn> = Arc::new(self.pool.clone());
        req.set_ext(conn.clone());
        let res = next.run(req).await;
        Ok(res)
    }
}

#[async_trait]
pub trait DieselRequestExt {
    async fn mysql_conn<'req>(
        &'req self,
    ) -> std::result::Result<PooledMysqlConn, diesel::r2d2::PoolError>;
    async fn mysql_pool_conn<'req>(
        &'req self,
    ) -> std::result::Result<&Arc<PoolMysqlConn>, diesel::r2d2::PoolError>;
}

#[async_trait]
impl<T: Clone + Send + Sync + 'static> DieselRequestExt for Request<T> {
    async fn mysql_conn<'req>(
        &'req self,
    ) -> std::result::Result<PooledMysqlConn, diesel::r2d2::PoolError> {
        let mysql_conn: &Arc<PoolMysqlConn> =
            self.ext().expect("You must install Diesel middleware");
        mysql_conn.get()
    }

    async fn mysql_pool_conn<'req>(
        &'req self,
    ) -> std::result::Result<&Arc<PoolMysqlConn>, diesel::r2d2::PoolError> {
        let mysql_conn: &Arc<PoolMysqlConn> =
            self.ext().expect("You must install Diesel middleware");
        Ok(mysql_conn)
    }
}
