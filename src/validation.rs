use sqlparser::dialect::{DatabricksDialect, MsSqlDialect};
use sqlparser::parser::Parser;
use sqlparser::ast::{ObjectName, ObjectNamePart, SetExpr, Statement};

/// Which backend dialect to use for SQL parsing.
#[derive(Debug, Clone, Copy)]
pub enum BackendDialect {
    SqlServer,
    Databricks,
}

/// Why a statement was denied.
#[derive(Debug, Clone)]
pub enum DenialKind {
    Dml,
    Ddl,
    Dcl,
    Operational,
    StoredProcedure,
    SelectInto,
    CteWrappedWrite,
    ParseFailure,
}

/// Allowlisted SQL Server system stored procedures known to be read-only.
const SAFE_PROCEDURES: &[&str] = &[
    // Catalog procedures (ODBC data dictionary)
    "sp_column_privileges",
    "sp_columns",
    "sp_databases",
    "sp_fkeys",
    "sp_pkeys",
    "sp_server_info",
    "sp_special_columns",
    "sp_sproc_columns",
    "sp_statistics",
    "sp_stored_procedures",
    "sp_table_privileges",
    "sp_tables",
    // Object/metadata procedures
    "sp_help",
    "sp_helptext",
    "sp_helpindex",
    "sp_helpconstraint",
    // Session/server info
    "sp_who",
    "sp_who2",
    "sp_spaceused",
    // Result set metadata
    "sp_describe_first_result_set",
    "sp_describe_undeclared_parameters",
];

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
        if let Some((kind, detail)) = is_denied_statement(stmt, dialect) {
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

/// Check if a statement is denied. Returns `Some((kind, detail))` if denied, `None` if allowed.
/// This is the deny-list core: unknown statement types fall through to `_ => None` (allowed).
fn is_denied_statement(stmt: &Statement, dialect: BackendDialect) -> Option<(DenialKind, String)> {
    match stmt {
        // DML
        Statement::Insert(_) => Some((DenialKind::Dml, "Denied: DML statement (INSERT)".into())),
        Statement::Update { .. } => Some((DenialKind::Dml, "Denied: DML statement (UPDATE)".into())),
        Statement::Delete(_) => Some((DenialKind::Dml, "Denied: DML statement (DELETE)".into())),
        Statement::Merge { .. } => Some((DenialKind::Dml, "Denied: DML statement (MERGE)".into())),

        // DDL — CREATE
        Statement::CreateTable { .. }
        | Statement::CreateView { .. }
        | Statement::CreateIndex(_)
        | Statement::CreateFunction(_)
        | Statement::CreateProcedure { .. }
        | Statement::CreateTrigger(_)
        | Statement::CreateSequence { .. }
        | Statement::CreateSchema { .. }
        | Statement::CreateDatabase { .. }
        | Statement::CreateType { .. }
        | Statement::CreateDomain(_)
        | Statement::CreateExtension(_)
        | Statement::CreateVirtualTable { .. }
        | Statement::CreateMacro { .. }
        | Statement::CreateSecret { .. }
        | Statement::CreateStage { .. }
        | Statement::CreateConnector(_)
        | Statement::CreatePolicy(_) => {
            Some((DenialKind::Ddl, "Denied: DDL statement (CREATE)".into()))
        }

        // DDL — ALTER
        Statement::AlterTable { .. }
        | Statement::AlterView { .. }
        | Statement::AlterSchema(_)
        | Statement::AlterIndex { .. }
        | Statement::AlterType(_)
        | Statement::AlterConnector { .. }
        | Statement::AlterPolicy(_) => {
            Some((DenialKind::Ddl, "Denied: DDL statement (ALTER)".into()))
        }

        // DDL — DROP
        Statement::Drop { .. }
        | Statement::DropFunction(_)
        | Statement::DropProcedure { .. }
        | Statement::DropTrigger(_)
        | Statement::DropExtension(_)
        | Statement::DropSecret { .. }
        | Statement::DropConnector { .. }
        | Statement::DropPolicy(_)
        | Statement::DropOperator(_)
        | Statement::DropOperatorFamily(_)
        | Statement::DropOperatorClass(_)
        | Statement::DropDomain(_) => {
            Some((DenialKind::Ddl, "Denied: DDL statement (DROP)".into()))
        }

        // DDL — other
        Statement::Truncate { .. } => Some((DenialKind::Ddl, "Denied: DDL statement (TRUNCATE)".into())),
        Statement::RenameTable(_) => Some((DenialKind::Ddl, "Denied: DDL statement (RENAME)".into())),

        // DCL
        Statement::Grant(_) => Some((DenialKind::Dcl, "Denied: DCL statement (GRANT)".into())),
        Statement::Revoke(_) => Some((DenialKind::Dcl, "Denied: DCL statement (REVOKE)".into())),
        Statement::Deny(_) => Some((DenialKind::Dcl, "Denied: DCL statement (DENY)".into())),
        Statement::CreateUser(_) => Some((DenialKind::Dcl, "Denied: DCL statement (CREATE USER)".into())),
        Statement::AlterUser(_) => Some((DenialKind::Dcl, "Denied: DCL statement (ALTER USER)".into())),
        Statement::CreateRole(_) => Some((DenialKind::Dcl, "Denied: DCL statement (CREATE ROLE)".into())),
        Statement::AlterRole { .. } => Some((DenialKind::Dcl, "Denied: DCL statement (ALTER ROLE)".into())),

        // Operational
        Statement::Copy { .. } => Some((DenialKind::Operational, "Denied: operational statement (COPY)".into())),
        Statement::CopyIntoSnowflake { .. } => Some((DenialKind::Operational, "Denied: operational statement (COPY INTO)".into())),
        Statement::LoadData { .. } => Some((DenialKind::Operational, "Denied: operational statement (LOAD DATA)".into())),
        Statement::Unload { .. } => Some((DenialKind::Operational, "Denied: operational statement (UNLOAD)".into())),
        Statement::Kill { .. } => Some((DenialKind::Operational, "Denied: operational statement (KILL)".into())),
        Statement::Flush { .. } => Some((DenialKind::Operational, "Denied: operational statement (FLUSH)".into())),
        Statement::Install { .. } => Some((DenialKind::Operational, "Denied: operational statement (INSTALL)".into())),
        Statement::AttachDatabase { .. } => Some((DenialKind::Operational, "Denied: operational statement (ATTACH)".into())),
        Statement::AttachDuckDBDatabase { .. } => Some((DenialKind::Operational, "Denied: operational statement (ATTACH)".into())),
        Statement::DetachDuckDBDatabase { .. } => Some((DenialKind::Operational, "Denied: operational statement (DETACH)".into())),

        // EXEC — check against safe procedure allowlist (SQL Server only)
        Statement::Execute { name, .. } => {
            if let Some(proc_name) = name
                && check_exec_allowlist(proc_name, dialect)
            {
                return None;
            }
            Some((
                DenialKind::StoredProcedure,
                "Denied: stored procedure execution".into(),
            ))
        }

        // Query — check for SELECT INTO and CTE-wrapped writes
        Statement::Query(query) => check_query_denial(&query.body),

        // Recursive — check nested statements in transactions, IF, WHILE
        Statement::StartTransaction { statements, .. } => {
            check_nested_statements(statements, dialect)
        }
        Statement::If(if_stmt) => {
            if let Some(d) = check_nested_statements(if_stmt.if_block.conditional_statements.statements(), dialect) {
                return Some(d);
            }
            for block in &if_stmt.elseif_blocks {
                if let Some(d) = check_nested_statements(block.conditional_statements.statements(), dialect) {
                    return Some(d);
                }
            }
            if let Some(else_block) = &if_stmt.else_block
                && let Some(d) = check_nested_statements(else_block.conditional_statements.statements(), dialect) {
                    return Some(d);
            }
            None
        }
        Statement::While(while_stmt) => {
            check_nested_statements(
                while_stmt.while_block.conditional_statements.statements(),
                dialect,
            )
        }

        // Deny-list default: everything not explicitly denied is allowed
        _ => None,
    }
}

/// Check if an EXEC procedure name is in the safe allowlist.
/// Returns `true` if the procedure is allowed (safe to execute).
/// Only applies to SQL Server dialect; all EXEC is denied on other dialects.
fn check_exec_allowlist(name: &ObjectName, dialect: BackendDialect) -> bool {
    if !matches!(dialect, BackendDialect::SqlServer) {
        return false;
    }

    // Extract the final segment of a potentially multi-part name
    // e.g., "master.dbo.sp_help" → "sp_help", "dbo.sp_help" → "sp_help"
    let proc_name = match name.0.last() {
        Some(ObjectNamePart::Identifier(ident)) => ident.value.to_lowercase(),
        _ => return false,
    };

    // sp_executesql is explicitly excluded — it can execute arbitrary SQL
    if proc_name == "sp_executesql" {
        return false;
    }

    SAFE_PROCEDURES.contains(&proc_name.as_str())
}

/// Check if a Query's body contains SELECT INTO or CTE-wrapped writes.
fn check_query_denial(body: &SetExpr) -> Option<(DenialKind, String)> {
    match body {
        SetExpr::Select(select) if select.into.is_some() => Some((
            DenialKind::SelectInto,
            "Denied: SELECT INTO (creates table)".into(),
        )),
        SetExpr::Query(query) => check_query_denial(&query.body),
        SetExpr::SetOperation { left, right, .. } => {
            check_query_denial(left).or_else(|| check_query_denial(right))
        }
        // CTE-wrapped writes
        SetExpr::Insert(_) => Some((
            DenialKind::CteWrappedWrite,
            "Denied: CTE-wrapped INSERT".into(),
        )),
        SetExpr::Update(_) => Some((
            DenialKind::CteWrappedWrite,
            "Denied: CTE-wrapped UPDATE".into(),
        )),
        SetExpr::Delete(_) => Some((
            DenialKind::CteWrappedWrite,
            "Denied: CTE-wrapped DELETE".into(),
        )),
        SetExpr::Merge(_) => Some((
            DenialKind::CteWrappedWrite,
            "Denied: CTE-wrapped MERGE".into(),
        )),
        // Safe query bodies
        _ => None,
    }
}

/// Check nested statements (owned Vec<Statement>). If any is denied, return the first denial.
fn check_nested_statements(
    statements: &[Statement],
    dialect: BackendDialect,
) -> Option<(DenialKind, String)> {
    for stmt in statements {
        if let Some(denial) = is_denied_statement(stmt, dialect) {
            return Some(denial);
        }
    }
    None
}

