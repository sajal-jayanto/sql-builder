# sql-builder

A small, dependency-free fluent builder for SQL `SELECT` statements, written in Rust.

```rust
use query_builder::{Order, QueryBuilder};

let sql = QueryBuilder::new()
  .select(["u.id", "u.name", "COUNT(o.id) AS order_count"])
  .from("users u")
  .left_join("orders o", "o.user_id = u.id")
  .where_("u.active = true")
  .where_in("u.role", ["'admin'", "'editor'"])
  .group_by(["u.id", "u.name"])
  .having("COUNT(o.id) > 0")
  .order_by("order_count", Order::Desc)
  .limit(10)
  .build()?;

// SELECT u.id, u.name, COUNT(o.id) AS order_count FROM users u
// LEFT JOIN orders o ON o.user_id = u.id
// WHERE u.active = true AND u.role IN ('admin', 'editor')
// GROUP BY u.id, u.name HAVING COUNT(o.id) > 0
// ORDER BY order_count DESC LIMIT 10
```

## Getting started

Requires Rust with edition 2024 support (1.85+).

```sh
cargo run    # prints the example query from src/main.rs
cargo test   # runs the unit tests in src/query_builder.rs
```

## Features

| Area | Methods |
| --- | --- |
| Select | `select`, `distinct` (no columns means `SELECT *`) |
| From | `from` |
| Joins | `inner_join`, `left_join`, `right_join`, `full_join`, `cross_join` |
| Where | `where_`, `and_where`, `or_where`, `where_in`, `where_not_in`, `where_between`, `where_null`, `where_not_null`, `where_exists` |
| Grouping | `group_by`, `having` (multiple calls are joined with `AND`) |
| Ordering and paging | `order_by` with `Order::Asc` / `Order::Desc`, `limit`, `offset` |
| Set operations | `union`, `union_all`, `intersect`, `except` |
| CTEs | `with`, `with_recursive` |
| Row locking | `for_update`, `for_share`, `skip_locked`, `no_wait` |

Every method takes `&mut self` and returns `&mut Self`, so calls chain. `build()` borrows the builder and returns `Result<String, &'static str>`.

### Adding clauses conditionally

Keep the builder in a `mut` variable and call methods on it whenever you need to:

```rust
let mut query = QueryBuilder::new();
query.from("users u");

if only_active {
  query.where_("u.active = true");
}
if let Some(role) = role {
  query.where_(format!("u.role = {role}"));
}

let sql = query.build()?;
```

Create the builder with `QueryBuilder::new()` on its own line first. `let q = QueryBuilder::new().from("users");` doesn't compile, because `from` returns a borrow of a temporary value. A chain that ends in `.build()`, or one passed straight in as a subquery, works fine.

### Subqueries, CTEs and set operations

`where_exists`, `with`, `with_recursive` and the set operations each take another `QueryBuilder`:

```rust
QueryBuilder::new()
  .with_recursive(
    "nums",
    QueryBuilder::new()
      .select(["1 AS n"])
      .union_all(QueryBuilder::new().select(["n + 1"]).from("nums").where_("n < 10")),
  )
  .from("nums")
  .build()?;
// WITH RECURSIVE nums AS (SELECT 1 AS n UNION ALL SELECT n + 1 FROM nums WHERE n < 10) SELECT * FROM nums
```

When set operations are used, `order_by`, `limit` and `offset` on the outer builder apply to the combined result.

### Build errors

`build()` returns an error when:

- `skip_locked` or `no_wait` is used without `for_update` or `for_share`
- row locking is combined with `union` / `intersect` / `except`
- a query passed to a set operation has its own `order_by`, `limit` or `offset`

## Things to know

- **Values are not escaped or parameterized.** Every string is inserted into the SQL as-is, which is why string literals are written with their quotes (`"'admin'"`). Never pass user input directly; use your database driver's bind parameters for values.
- **Conditions are not grouped.** `where_`, `and_where` and `or_where` are joined in order with no parentheses, so the database's normal precedence applies (`AND` binds tighter than `OR`). `a AND b OR c` means `(a AND b) OR c`. For other groupings, write the parentheses into a single condition, such as `.where_("(a OR b)")`.
- **Empty `IN` lists** are handled: `where_in` with no values becomes `1 = 0` and `where_not_in` becomes `1 = 1`, since `IN ()` is invalid SQL.
- Only `SELECT` is supported. The locking and `FULL JOIN` syntax follows PostgreSQL; some databases don't support all of it.

## Project layout

```
src/
  main.rs            example usage
  query_builder.rs   the builder and its tests
```
