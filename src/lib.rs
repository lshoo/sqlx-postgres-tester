use std::{path::Path, thread};

use sqlx::{migrate::Migrator, Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct TestPg {
    server_url: String,
    pub dbname: String,
}

impl Default for TestPg {
    fn default() -> Self {
        let server_url = "postgres://postgres:postgres@localhost:5432";
        let migration_path = "./migrations";

        Self::new(server_url, migration_path)
    }
}

impl TestPg {
    pub fn new(server_url: impl Into<String>, migration_path: impl Into<String>) -> Self {
        let server_url = server_url.into();
        let uuid = Uuid::new_v4();
        let dbname = format!("test_{}", uuid);

        let tdb = Self {
            server_url,
            dbname: dbname.clone(),
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
                conn.execute(format!(r#"CREATE DATABASE "{}""#, dbname).as_str())
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
        self.server_url.clone()
    }

    pub async fn get_pool(&self) -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url())
            .await
            .unwrap()
    }
}

impl Drop for TestPg {
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
        let tdb = TestPg::default();
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
