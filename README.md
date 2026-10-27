# rs_pg_db_rest

 Version: 0.9.1

 date    : 2026/10/25
 
 update :

***

Rust Axum API , Postgres DB example

* rustc 1.99.1

***
* .env
```
DATABASE_URL=postgresql://root:admin@localhost:5432/mydb
```

***
* Table: table.sql

***
* build
```
cargo build --release
```

***
* start
```
./target/release/pg_db_rest
```

***
* Test-code
* INSERT

```
curl -X POST http://localhost:8888/api/update \
  -H "Content-Type: application/json" \
  -d "{\"table\": \"todos\", \"sql\": \"INSERT INTO todos (title) VALUES ('title-1');\"}"
```

* select
```
curl -X POST http://localhost:8888/api/select \
  -H "Content-Type: application/json" \
  -d '{"table": "todos", "sql": "SELECT * FROM todos;"}'

```

***
### blog

https://zenn.dev/knaka0209/scraps/e703f4a3849834


***
