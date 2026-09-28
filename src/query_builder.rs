#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
  Asc,
  Desc,
}

#[derive(Debug, Clone, Copy)]
enum JoinKind {
  Inner,
  Left,
  Right,
  Full,
  Cross,
}

impl JoinKind {
  fn keyword(self) -> &'static str {
    match self {
      JoinKind::Inner => "INNER JOIN",
      JoinKind::Left => "LEFT JOIN",
      JoinKind::Right => "RIGHT JOIN",
      JoinKind::Full => "FULL JOIN",
      JoinKind::Cross => "CROSS JOIN",
    }
  }
}

#[derive(Debug, Clone)]
struct Join {
  kind: JoinKind,
  table: String,
  on: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum Connector {
  And,
  Or,
}

#[derive(Debug, Clone)]
enum Condition {
  Raw(String),
  Exists(Box<QueryBuilder>),
}

#[derive(Debug, Clone, Copy)]
enum SetOp {
  Union,
  UnionAll,
  Intersect,
  Except,
}

impl SetOp {
  fn keyword(self) -> &'static str {
    match self {
      SetOp::Union => "UNION",
      SetOp::UnionAll => "UNION ALL",
      SetOp::Intersect => "INTERSECT",
      SetOp::Except => "EXCEPT",
    }
  }
}

#[derive(Debug, Clone, Copy)]
enum LockMode {
  Update,
  Share,
}

#[derive(Debug, Clone, Copy)]
enum LockWait {
  SkipLocked,
  NoWait,
}

#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
  ctes: Vec<(String, QueryBuilder)>,
  recursive: bool,
  distinct: bool,
  columns: Vec<String>,
  from: Option<String>,
  joins: Vec<Join>,
  conditions: Vec<(Connector, Condition)>,
  group_by: Vec<String>,
  having: Vec<String>,
  set_ops: Vec<(SetOp, QueryBuilder)>,
  order_by: Vec<(String, Order)>,
  limit: Option<u64>,
  offset: Option<u64>,
  lock: Option<LockMode>,
  lock_wait: Option<LockWait>,
}

impl QueryBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  // ---------- SELECT ----------

  pub fn select<I, S>(mut self, columns: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.columns.extend(columns.into_iter().map(Into::into));
    self
  }

  pub fn distinct(mut self) -> Self {
    self.distinct = true;
    self
  }

  // ---------- FROM ----------

  pub fn from(mut self, table: impl Into<String>) -> Self {
    self.from = Some(table.into());
    self
  }

  // ---------- JOINS ----------

  fn join(mut self, kind: JoinKind, table: String, on: Option<String>) -> Self {
    self.joins.push(Join { kind, table, on });
    self
  }

  pub fn inner_join(self, table: impl Into<String>, on: impl Into<String>) -> Self {
    self.join(JoinKind::Inner, table.into(), Some(on.into()))
  }

  pub fn left_join(self, table: impl Into<String>, on: impl Into<String>) -> Self {
    self.join(JoinKind::Left, table.into(), Some(on.into()))
  }

  pub fn right_join(self, table: impl Into<String>, on: impl Into<String>) -> Self {
    self.join(JoinKind::Right, table.into(), Some(on.into()))
  }

  pub fn full_join(self, table: impl Into<String>, on: impl Into<String>) -> Self {
    self.join(JoinKind::Full, table.into(), Some(on.into()))
  }

  pub fn cross_join(self, table: impl Into<String>) -> Self {
    self.join(JoinKind::Cross, table.into(), None)
  }

  // ---------- WHERE ----------
  // `where` is a Rust keyword, hence the trailing underscore.

  fn push_condition(mut self, connector: Connector, condition: Condition) -> Self {
    self.conditions.push((connector, condition));
    self
  }

  pub fn where_(self, condition: impl Into<String>) -> Self {
    self.push_condition(Connector::And, Condition::Raw(condition.into()))
  }

  pub fn and_where(self, condition: impl Into<String>) -> Self {
    self.push_condition(Connector::And, Condition::Raw(condition.into()))
  }

  pub fn or_where(self, condition: impl Into<String>) -> Self {
    self.push_condition(Connector::Or, Condition::Raw(condition.into()))
  }

  pub fn where_in<I, S>(self, column: impl Into<String>, values: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let values: Vec<String> = values.into_iter().map(Into::into).collect();
    // `x IN ()` is invalid SQL; an empty list can never match.
    let condition = if values.is_empty() {
      "1 = 0".to_string()
    } else {
      format!("{} IN ({})", column.into(), values.join(", "))
    };
    self.and_where(condition)
  }

  pub fn where_not_in<I, S>(self, column: impl Into<String>, values: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let values: Vec<String> = values.into_iter().map(Into::into).collect();
    // `x NOT IN ()` is invalid SQL; an empty list always matches.
    let condition = if values.is_empty() {
      "1 = 1".to_string()
    } else {
      format!("{} NOT IN ({})", column.into(), values.join(", "))
    };
    self.and_where(condition)
  }

  pub fn where_between(
    self,
    column: impl Into<String>,
    low: impl Into<String>,
    high: impl Into<String>,
  ) -> Self {
    let condition = format!("{} BETWEEN {} AND {}", column.into(), low.into(), high.into());
    self.and_where(condition)
  }

  pub fn where_null(self, column: impl Into<String>) -> Self {
    let condition = format!("{} IS NULL", column.into());
    self.and_where(condition)
  }

  pub fn where_not_null(self, column: impl Into<String>) -> Self {
    let condition = format!("{} IS NOT NULL", column.into());
    self.and_where(condition)
  }

  pub fn where_exists(self, subquery: QueryBuilder) -> Self {
    self.push_condition(Connector::And, Condition::Exists(Box::new(subquery)))
  }

  // ---------- GROUP BY / HAVING ----------

  pub fn group_by<I, S>(mut self, columns: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.group_by.extend(columns.into_iter().map(Into::into));
    self
  }

  pub fn having(mut self, condition: impl Into<String>) -> Self {
    self.having.push(condition.into());
    self
  }

  // ---------- ORDER BY / LIMIT / OFFSET ----------

  pub fn order_by(mut self, column: impl Into<String>, order: Order) -> Self {
    self.order_by.push((column.into(), order));
    self
  }

  pub fn limit(mut self, limit: u64) -> Self {
    self.limit = Some(limit);
    self
  }

  pub fn offset(mut self, offset: u64) -> Self {
    self.offset = Some(offset);
    self
  }

  // ---------- SET OPERATIONS ----------
  // ORDER BY / LIMIT / OFFSET on the outer builder apply to the combined result.

  fn set_op(mut self, op: SetOp, query: QueryBuilder) -> Self {
    self.set_ops.push((op, query));
    self
  }

  pub fn union(self, query: QueryBuilder) -> Self {
    self.set_op(SetOp::Union, query)
  }

  pub fn union_all(self, query: QueryBuilder) -> Self {
    self.set_op(SetOp::UnionAll, query)
  }

  pub fn intersect(self, query: QueryBuilder) -> Self {
    self.set_op(SetOp::Intersect, query)
  }

  pub fn except(self, query: QueryBuilder) -> Self {
    self.set_op(SetOp::Except, query)
  }

  // ---------- CTEs ----------

  pub fn with(mut self, name: impl Into<String>, query: QueryBuilder) -> Self {
    self.ctes.push((name.into(), query));
    self
  }

  // SQL marks the whole WITH clause as RECURSIVE, not individual CTEs.
  pub fn with_recursive(mut self, name: impl Into<String>, query: QueryBuilder) -> Self {
    self.recursive = true;
    self.ctes.push((name.into(), query));
    self
  }

  // ---------- LOCKING ----------

  pub fn for_update(mut self) -> Self {
    self.lock = Some(LockMode::Update);
    self
  }

  pub fn for_share(mut self) -> Self {
    self.lock = Some(LockMode::Share);
    self
  }

  pub fn skip_locked(mut self) -> Self {
    self.lock_wait = Some(LockWait::SkipLocked);
    self
  }

  pub fn no_wait(mut self) -> Self {
    self.lock_wait = Some(LockWait::NoWait);
    self
  }

  // ---------- BUILD ----------

  pub fn build(self) -> Result<String, &'static str> {
    if self.lock_wait.is_some() && self.lock.is_none() {
      return Err("skip_locked/no_wait requires for_update or for_share");
    }
    if self.lock.is_some() && !self.set_ops.is_empty() {
      return Err("row locking cannot be combined with union/intersect/except");
    }

    let mut sql = String::new();

    if !self.ctes.is_empty() {
      sql.push_str(if self.recursive { "WITH RECURSIVE " } else { "WITH " });
      let mut ctes = Vec::with_capacity(self.ctes.len());
      for (name, query) in self.ctes {
        ctes.push(format!("{} AS ({})", name, query.build()?));
      }
      sql.push_str(&ctes.join(", "));
      sql.push(' ');
    }

    sql.push_str("SELECT ");
    if self.distinct {
      sql.push_str("DISTINCT ");
    }
    if self.columns.is_empty() {
      sql.push('*');
    } else {
      sql.push_str(&self.columns.join(", "));
    }

    if let Some(table) = self.from {
      sql.push_str(" FROM ");
      sql.push_str(&table);
    }

    for join in self.joins {
      sql.push(' ');
      sql.push_str(join.kind.keyword());
      sql.push(' ');
      sql.push_str(&join.table);
      if let Some(on) = join.on {
        sql.push_str(" ON ");
        sql.push_str(&on);
      }
    }

    for (i, (connector, condition)) in self.conditions.into_iter().enumerate() {
      sql.push_str(match (i, connector) {
        (0, _) => " WHERE ",
        (_, Connector::And) => " AND ",
        (_, Connector::Or) => " OR ",
      });
      match condition {
        Condition::Raw(expr) => sql.push_str(&expr),
        Condition::Exists(subquery) => {
          sql.push_str("EXISTS (");
          sql.push_str(&subquery.build()?);
          sql.push(')');
        }
      }
    }

    if !self.group_by.is_empty() {
      sql.push_str(" GROUP BY ");
      sql.push_str(&self.group_by.join(", "));
    }

    if !self.having.is_empty() {
      sql.push_str(" HAVING ");
      sql.push_str(&self.having.join(" AND "));
    }

    for (op, query) in self.set_ops {
      if !query.order_by.is_empty() || query.limit.is_some() || query.offset.is_some() {
        return Err("queries combined with union/intersect/except cannot have order_by/limit/offset");
      }
      sql.push(' ');
      sql.push_str(op.keyword());
      sql.push(' ');
      sql.push_str(&query.build()?);
    }

    if !self.order_by.is_empty() {
      let columns: Vec<String> = self
        .order_by
        .into_iter()
        .map(|(column, order)| match order {
          Order::Asc => format!("{column} ASC"),
          Order::Desc => format!("{column} DESC"),
        })
        .collect();
      sql.push_str(" ORDER BY ");
      sql.push_str(&columns.join(", "));
    }

    if let Some(limit) = self.limit {
      sql.push_str(&format!(" LIMIT {limit}"));
    }

    if let Some(offset) = self.offset {
      sql.push_str(&format!(" OFFSET {offset}"));
    }

    if let Some(lock) = self.lock {
      sql.push_str(match lock {
        LockMode::Update => " FOR UPDATE",
        LockMode::Share => " FOR SHARE",
      });
    }

    if let Some(wait) = self.lock_wait {
      sql.push_str(match wait {
        LockWait::SkipLocked => " SKIP LOCKED",
        LockWait::NoWait => " NOWAIT",
      });
    }

    Ok(sql)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn select_all_by_default() {
    let sql = QueryBuilder::new().from("users").build().unwrap();
    assert_eq!(sql, "SELECT * FROM users");
  }

  #[test]
  fn full_select() {
    let sql = QueryBuilder::new()
      .select(["u.id", "u.name", "COUNT(o.id) AS orders"])
      .distinct()
      .from("users u")
      .left_join("orders o", "o.user_id = u.id")
      .where_("u.active = true")
      .where_in("u.role", ["'admin'", "'editor'"])
      .or_where("u.id = 1")
      .group_by(["u.id", "u.name"])
      .having("COUNT(o.id) > 5")
      .order_by("orders", Order::Desc)
      .limit(10)
      .offset(20)
      .build()
      .unwrap();
    assert_eq!(
      sql,
      "SELECT DISTINCT u.id, u.name, COUNT(o.id) AS orders FROM users u \
       LEFT JOIN orders o ON o.user_id = u.id \
       WHERE u.active = true AND u.role IN ('admin', 'editor') OR u.id = 1 \
       GROUP BY u.id, u.name HAVING COUNT(o.id) > 5 \
       ORDER BY orders DESC LIMIT 10 OFFSET 20"
    );
  }

  #[test]
  fn where_helpers() {
    let sql = QueryBuilder::new()
      .from("t")
      .where_not_in("a", ["1", "2"])
      .where_between("b", "10", "20")
      .where_null("c")
      .where_not_null("d")
      .where_exists(QueryBuilder::new().select(["1"]).from("x").where_("x.t_id = t.id"))
      .build()
      .unwrap();
    assert_eq!(
      sql,
      "SELECT * FROM t WHERE a NOT IN (1, 2) AND b BETWEEN 10 AND 20 \
       AND c IS NULL AND d IS NOT NULL \
       AND EXISTS (SELECT 1 FROM x WHERE x.t_id = t.id)"
    );
  }

  #[test]
  fn empty_in_list() {
    let values: [&str; 0] = [];
    let sql = QueryBuilder::new().from("t").where_in("a", values).build().unwrap();
    assert_eq!(sql, "SELECT * FROM t WHERE 1 = 0");
  }

  #[test]
  fn joins() {
    let sql = QueryBuilder::new()
      .from("a")
      .inner_join("b", "b.a_id = a.id")
      .right_join("c", "c.a_id = a.id")
      .full_join("d", "d.a_id = a.id")
      .cross_join("e")
      .build()
      .unwrap();
    assert_eq!(
      sql,
      "SELECT * FROM a INNER JOIN b ON b.a_id = a.id RIGHT JOIN c ON c.a_id = a.id \
       FULL JOIN d ON d.a_id = a.id CROSS JOIN e"
    );
  }

  #[test]
  fn set_operations() {
    let sql = QueryBuilder::new()
      .select(["id"])
      .from("a")
      .union(QueryBuilder::new().select(["id"]).from("b"))
      .union_all(QueryBuilder::new().select(["id"]).from("c"))
      .intersect(QueryBuilder::new().select(["id"]).from("d"))
      .except(QueryBuilder::new().select(["id"]).from("e"))
      .order_by("id", Order::Asc)
      .build()
      .unwrap();
    assert_eq!(
      sql,
      "SELECT id FROM a UNION SELECT id FROM b UNION ALL SELECT id FROM c \
       INTERSECT SELECT id FROM d EXCEPT SELECT id FROM e ORDER BY id ASC"
    );
  }

  #[test]
  fn ctes() {
    let sql = QueryBuilder::new()
      .with("active", QueryBuilder::new().from("users").where_("active"))
      .with_recursive(
        "nums",
        QueryBuilder::new()
          .select(["1 AS n"])
          .union_all(QueryBuilder::new().select(["n + 1"]).from("nums").where_("n < 10")),
      )
      .from("nums")
      .build()
      .unwrap();
    assert_eq!(
      sql,
      "WITH RECURSIVE active AS (SELECT * FROM users WHERE active), \
       nums AS (SELECT 1 AS n UNION ALL SELECT n + 1 FROM nums WHERE n < 10) \
       SELECT * FROM nums"
    );
  }

  #[test]
  fn locking() {
    let sql = QueryBuilder::new()
      .from("jobs")
      .limit(1)
      .for_update()
      .skip_locked()
      .build()
      .unwrap();
    assert_eq!(sql, "SELECT * FROM jobs LIMIT 1 FOR UPDATE SKIP LOCKED");

    let sql = QueryBuilder::new().from("jobs").for_share().no_wait().build().unwrap();
    assert_eq!(sql, "SELECT * FROM jobs FOR SHARE NOWAIT");
  }

  #[test]
  fn invalid_combinations() {
    assert!(QueryBuilder::new().from("t").skip_locked().build().is_err());
    assert!(
      QueryBuilder::new()
        .from("a")
        .union(QueryBuilder::new().from("b"))
        .for_update()
        .build()
        .is_err()
    );
    assert!(
      QueryBuilder::new()
        .from("a")
        .union(QueryBuilder::new().from("b").limit(1))
        .build()
        .is_err()
    );
  }
}
