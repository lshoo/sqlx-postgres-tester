![](https://github.com/lshoo/sqlx-postgres-tester/workflows/build/badge.svg)

# sqlx-postgres-tester

This is a tool to test sqlx with postgres and tokio runtime only.

## How to use it

First, create a `TestDb` struct instance in tests. It will automatically create database and a connection pool.
Then get the connection string or   connection pool from it to use in codes.
Finally, when `TestDb` gets dropped, it will automatically drop the database.


```rust
#[tokio::test]
async fn test_db_should_work() {
    let tdb = TestDb::new("localhost", 5432, "postgres", "postgres", "./migrations");
    let pool = tdb.get_pool().await;
    // do something with pool
}

```

## License

This project is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2023 lshoo
