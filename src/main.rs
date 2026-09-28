mod query_builder;

use query_builder::{Order, QueryBuilder};

fn main() {
  let only_active = true;
  let role: Option<&str> = Some("'admin'");

  let mut query = QueryBuilder::new();
  query
    .select(["u.id", "u.name", "COUNT(o.id) AS order_count"])
    .from("users u")
    .left_join("orders o", "o.user_id = u.id");

  if only_active {
    query.where_("u.active = true");
  }

  if let Some(role) = role {
    query.where_(format!("u.role = {role}"));
  }

  query
    .group_by(["u.id", "u.name"])
    .order_by("order_count", Order::Desc)
    .limit(10);

  let sql = query.build().expect("Failed to build query");

  println!("{sql}");
}
