use sqlparser::dialect::{DatabricksDialect, MsSqlDialect};
use sqlparser::parser::Parser;
use sqlparser::ast::{SetExpr, Statement};

/// Which backend dialect to use for SQL parsing.
#[derive(Debug, Clone, Copy)]
pub enum BackendDialect {
    SqlServer,
    Databricks,
}

/// Why a statement was denied.
#[derive(Debug, Clone)]
pub enum DenialKind {
    WriteStatement,
    SelectInto,
    CteWrappedWrite,
    StoredProcedure,
    ParseFailure,
    Unrecognized,
}

/// A single denial reason.
#[derive(Debug, Clone)]
pub struct DenialReason {
    pub statement_index: usize,
    pub kind: DenialKind,
    pub detail: String,
}

/// Outcome of read-only query validation.
#[derive(Debug)]
pub enum ValidationResult {
    Safe,
    Denied { reasons: Vec<DenialReason> },
}

/// Validate that all SQL statements are read-only.
pub fn validate(sql: &str, dialect: BackendDialect) -> ValidationResult {
    let statements = match dialect {
        BackendDialect::SqlServer => {
            Parser::parse_sql(&MsSqlDialect {}, sql)
        }
        BackendDialect::Databricks => {
            Parser::parse_sql(&DatabricksDialect {}, sql)
        }
    };

    let statements = match statements {
        Ok(stmts) => stmts,
        Err(e) => {
            return ValidationResult::Denied {
                reasons: vec![DenialReason {
                    statement_index: 0,
                    kind: DenialKind::ParseFailure,
                    detail: format!("cannot verify query safety: {}", e),
                }],
            };
        }
    };

    let mut reasons = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        if !is_safe_statement(stmt) {
            let (kind, detail) = classify_denial(stmt);
            reasons.push(DenialReason {
                statement_index: i,
                kind,
                detail,
            });
        }
    }

    if reasons.is_empty() {
        ValidationResult::Safe
    } else {
        ValidationResult::Denied { reasons }
    }
}

fn is_safe_statement(stmt: &Statement) -> bool {
    match stmt {
        Statement::Query(query) => is_safe_query_body(&query.body),
        Statement::ExplainTable { .. }
        | Statement::Explain { .. } => true,
        Statement::ShowTables { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowVariable { .. } => true,
        Statement::Use(_) => true,
        _ => false,
    }
}

fn is_safe_query_body(body: &SetExpr) -> bool {
    match body {
        SetExpr::Select(select) => select.into.is_none(),
        SetExpr::Query(query) => is_safe_query_body(&query.body),
        SetExpr::SetOperation { left, right, .. } => {
            is_safe_query_body(left) && is_safe_query_body(right)
        }
        SetExpr::Values(_) => true,
        SetExpr::Table(_) => true,
        // INSERT, UPDATE, DELETE, MERGE wrapped in CTE
        SetExpr::Insert(_) | SetExpr::Update(_) => false,
        _ => false,
    }
}

fn classify_denial(stmt: &Statement) -> (DenialKind, String) {
    match stmt {
        Statement::Insert(_) => (
            DenialKind::WriteStatement,
            "query would modify state: INSERT".to_string(),
        ),
        Statement::Update { .. } => (
            DenialKind::WriteStatement,
            "query would modify state: UPDATE".to_string(),
        ),
        Statement::Delete(_) => (
            DenialKind::WriteStatement,
            "query would modify state: DELETE".to_string(),
        ),
        Statement::Drop { .. } => (
            DenialKind::WriteStatement,
            "query would modify state: DROP".to_string(),
        ),
        Statement::CreateTable { .. } | Statement::CreateView { .. } => (
            DenialKind::WriteStatement,
            "query would modify state: DDL".to_string(),
        ),
        Statement::AlterTable { .. } => (
            DenialKind::WriteStatement,
            "query would modify state: ALTER".to_string(),
        ),
        Statement::Truncate { .. } => (
            DenialKind::WriteStatement,
            "query would modify state: TRUNCATE".to_string(),
        ),
        Statement::Merge { .. } => (
            DenialKind::WriteStatement,
            "query would modify state: MERGE".to_string(),
        ),
        Statement::Execute { .. } => (
            DenialKind::StoredProcedure,
            "stored procedure execution is not allowed in read-only mode".to_string(),
        ),
        Statement::Query(query) => {
            // This is a query that failed is_safe_query_body — likely SELECT INTO or CTE-wrapped write
            classify_query_denial(&query.body)
        }
        _ => (
            DenialKind::Unrecognized,
            "unrecognized statement type — denied by default".to_string(),
        ),
    }
}

fn classify_query_denial(body: &SetExpr) -> (DenialKind, String) {
    match body {
        SetExpr::Select(select) if select.into.is_some() => (
            DenialKind::SelectInto,
            "SELECT INTO would create a table".to_string(),
        ),
        SetExpr::Insert(_) => (
            DenialKind::CteWrappedWrite,
            "CTE-wrapped INSERT is not allowed in read-only mode".to_string(),
        ),
        SetExpr::Update(_) => (
            DenialKind::CteWrappedWrite,
            "CTE-wrapped UPDATE is not allowed in read-only mode".to_string(),
        ),
        SetExpr::Delete(_) => (
            DenialKind::CteWrappedWrite,
            "CTE-wrapped DELETE is not allowed in read-only mode".to_string(),
        ),
        SetExpr::Merge(_) => (
            DenialKind::CteWrappedWrite,
            "CTE-wrapped MERGE is not allowed in read-only mode".to_string(),
        ),
        _ => (
            DenialKind::Unrecognized,
            "query contains unsafe operations".to_string(),
        ),
    }
}
