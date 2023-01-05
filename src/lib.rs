use std::{path::Path, thread};

use sqlx::{migrate::Migrator, Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct TestDb {
    host: String,
    port: u16,
    user: String,
    password: String,
    dbname: String,
}

impl TestDb {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        migration_path: impl Into<String>,
    ) -> Self {
        let host = host.into();
        let user = user.into();
        let password = password.into();
        let uuid = Uuid::new_v4();
        let dbname = format!("test_{}", uuid);
        let dbname_cloned = dbname.clone();

        let tdb = Self {
            host,
            port,
            user,
            password,
            dbname,
        };

        let server_url = tdb.server_url();
        let url = tdb.url();
        let migration_path = migration_path.into();

        // create database dbname
        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                // use server url to create database
                let mut conn = PgConnection::connect(&server_url).await.unwrap();
                conn.execute(format!(r#"CREATE DATABASE "{}""#, dbname_cloned).as_str())
                    .await
                    .expect("Error while create database");

                let mut conn = PgConnection::connect(&url).await.unwrap();
                let m = Migrator::new(Path::new(&migration_path)).await.unwrap();
                m.run(&mut conn).await.unwrap();
            });
        })
        .join()
        .expect("failed to create database");

        tdb
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }

    pub fn server_url(&self) -> String {
        if self.password.is_empty() {
            format!("postgres://{}@{}:{}", self.user, self.host, self.port,)
        } else {
            format!(
                "postgres://{}:{}@{}:{}",
                self.user, self.password, self.host, self.port,
            )
        }
    }

    pub async fn get_pool(&self) -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url())
            .await
            .unwrap()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let server_url = self.server_url();
        let dbname = self.dbname.clone();

        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = sqlx::PgConnection::connect(&server_url).await.unwrap();
                // terminate existing connections
                sqlx::query(&format!(r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = '{}'"#, dbname))
                    .execute(&mut conn)
                    .await
                    .expect("Terminate all other connections");

                conn
                    .execute(format!(r#"DROP DATABASE "{}""#, dbname).as_str())
                    .await
                    .expect("Error while drop database");
            });
        })
        .join()
        .expect("failed to Drop database");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_db_create_and_drop_should_work() {
        let tdb = TestDb::new("localhost", 5432, "postgres", "postgres", "./migrations");
        let pool = tdb.get_pool().await;

        sqlx::query("INSERT INTO todos (title) VALUES('todo1')")
            .execute(&pool)
            .await
            .unwrap();

        let (id, title) = sqlx::query_as::<_, (i32, String)>("SELECT id, title FROM todos")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(id, 1);
        assert_eq!(title, "todo1");
    }
}
