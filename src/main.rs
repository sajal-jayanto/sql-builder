mod query_builder;

use query_builder::{Order, QueryBuilder};

fn main() {
  let sql = QueryBuilder::new()
    // .with("recent_orders", QueryBuilder::new().from("orders").where_("created_at > NOW() - INTERVAL '30 days'"))
    .select(["u.id", "u.name", "COUNT(o.id) AS order_count"])
    .distinct()
    .from("users u")
    .left_join("recent_orders o", "o.user_id = u.id")
    .where_("u.active = true")
    .where_in("u.role", ["'admin'", "'editor'"])
    .where_not_null("u.email")
    .or_where("u.id = 1")
    .group_by(["u.id", "u.name"])
    .having("COUNT(o.id) > 0")
    .order_by("order_count", Order::Desc)
    .limit(10)
    .offset(0)
    .build()
    .expect("Failed to build query");

  println!("{sql}");
}