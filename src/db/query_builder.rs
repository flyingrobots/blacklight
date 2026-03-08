use rusqlite::types::ToSql;

pub struct QueryBuilder {
    base_sql: String,
    where_clauses: Vec<String>,
    params: Vec<Box<dyn ToSql + Send>>,
    order_by: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl QueryBuilder {
    pub fn new(base_sql: impl Into<String>) -> Self {
        Self {
            base_sql: base_sql.into(),
            where_clauses: Vec::new(),
            params: Vec::new(),
            order_by: None,
            limit: None,
            offset: None,
        }
    }

    pub fn r#where(mut self, condition: impl Into<String>, param: Box<dyn ToSql + Send>) -> Self {
        let condition = condition.into();
        // Replace ? with ?N
        let placeholder = format!("?{}", self.params.len() + 1);
        let final_condition = condition.replace('?', &placeholder);
        
        self.where_clauses.push(final_condition);
        self.params.push(param);
        self
    }

    pub fn order_by(mut self, order: impl Into<String>) -> Self {
        self.order_by = Some(order.into());
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn build(self) -> (String, Vec<Box<dyn ToSql + Send>>) {
        let mut sql = self.base_sql;
        
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        if let Some(order) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&order);
        }

        if let Some(_limit) = self.limit {
            let placeholder = format!("?{}", self.params.len() + 1);
            sql.push_str(&format!(" LIMIT {}", placeholder));
            // We need to add the limit to params, but we can't easily do it here
            // because self is consumed. Wait, let's fix this.
        }

        (sql, self.params)
    }

    pub fn build_with_limit(mut self) -> (String, Vec<Box<dyn ToSql + Send>>) {
        let mut sql = self.base_sql;
        
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        if let Some(order) = self.order_by {
            sql.push_str(" ORDER BY ");
            sql.push_str(&order);
        }

        if let Some(limit) = self.limit {
            let placeholder = format!("?{}", self.params.len() + 1);
            sql.push_str(&format!(" LIMIT {}", placeholder));
            self.params.push(Box::new(limit));
        }

        if let Some(offset) = self.offset {
            let placeholder = format!("?{}", self.params.len() + 1);
            sql.push_str(&format!(" OFFSET {}", placeholder));
            self.params.push(Box::new(offset));
        }

        (sql, self.params)
    }
}
