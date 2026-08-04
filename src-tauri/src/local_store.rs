use crate::id::new_object_id;
use crate::model::{BatchDetail, MigrationAudit, MigrationBatch, MigrationRow};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{Map, Value};
use std::path::Path;
use std::sync::Mutex;

pub struct LocalStore {
    connection: Mutex<Connection>,
}

impl LocalStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                r#"
                PRAGMA journal_mode = WAL;
                PRAGMA foreign_keys = ON;
                CREATE TABLE IF NOT EXISTS migration_batch (
                    batch_id TEXT PRIMARY KEY,
                    batch_name TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_description TEXT NOT NULL DEFAULT '',
                    conflict_strategy TEXT NOT NULL DEFAULT 'REUSE',
                    allow_create_factory INTEGER NOT NULL DEFAULT 0,
                    idempotency_key TEXT NOT NULL DEFAULT '',
                    mapping_json TEXT NOT NULL,
                    status TEXT NOT NULL,
                    total_count INTEGER NOT NULL DEFAULT 0,
                    valid_count INTEGER NOT NULL DEFAULT 0,
                    success_count INTEGER NOT NULL DEFAULT 0,
                    fail_count INTEGER NOT NULL DEFAULT 0,
                    skip_count INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    finished_at TEXT
                );
                CREATE UNIQUE INDEX IF NOT EXISTS uk_mig_batch_idem
                    ON migration_batch(idempotency_key) WHERE idempotency_key <> '';
                CREATE TABLE IF NOT EXISTS migration_row (
                    row_id TEXT PRIMARY KEY,
                    batch_id TEXT NOT NULL,
                    row_no INTEGER NOT NULL,
                    source_key TEXT NOT NULL,
                    source_hash TEXT NOT NULL,
                    status TEXT NOT NULL,
                    raw_json TEXT NOT NULL,
                    normalized_json TEXT NOT NULL,
                    error_code TEXT NOT NULL DEFAULT '',
                    error_message TEXT NOT NULL DEFAULT '',
                    id_med TEXT NOT NULL DEFAULT '',
                    id_med_unit TEXT NOT NULL DEFAULT '',
                    id_fac TEXT NOT NULL DEFAULT '',
                    id_med_pro TEXT NOT NULL DEFAULT '',
                    retry_count INTEGER NOT NULL DEFAULT 0,
                    updated_at TEXT NOT NULL,
                    FOREIGN KEY(batch_id) REFERENCES migration_batch(batch_id),
                    UNIQUE(batch_id, row_no)
                );
                CREATE INDEX IF NOT EXISTS ix_mig_row_batch_status ON migration_row(batch_id, status);
                CREATE TABLE IF NOT EXISTS migration_audit (
                    audit_id TEXT PRIMARY KEY,
                    batch_id TEXT NOT NULL,
                    row_id TEXT NOT NULL DEFAULT '',
                    trace_id TEXT NOT NULL,
                    operation TEXT NOT NULL,
                    target_table TEXT NOT NULL,
                    target_id TEXT NOT NULL DEFAULT '',
                    result TEXT NOT NULL,
                    before_json TEXT NOT NULL DEFAULT 'null',
                    after_json TEXT NOT NULL DEFAULT 'null',
                    message TEXT NOT NULL DEFAULT '',
                    operator_id TEXT NOT NULL DEFAULT '',
                    operated_at TEXT NOT NULL,
                    FOREIGN KEY(batch_id) REFERENCES migration_batch(batch_id)
                );
                CREATE INDEX IF NOT EXISTS ix_mig_audit_batch_time ON migration_audit(batch_id, operated_at DESC);
                "#,
            )
            .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn find_by_idempotency_key(&self, key: &str) -> Result<Option<String>, String> {
        if key.is_empty() {
            return Ok(None);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .query_row(
                "SELECT batch_id FROM migration_batch WHERE idempotency_key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn insert_batch(&self, batch: &MigrationBatch, mapping_json: &str) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_batch(
                batch_id,batch_name,source_type,source_name,source_description,conflict_strategy,
                allow_create_factory,idempotency_key,mapping_json,status,total_count,valid_count,
                success_count,fail_count,skip_count,created_at,updated_at,finished_at
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
                params![
                    batch.batch_id,
                    batch.batch_name,
                    batch.source_type,
                    batch.source_name,
                    batch.source_description,
                    batch.conflict_strategy,
                    batch.allow_create_factory as i32,
                    batch.idempotency_key,
                    mapping_json,
                    batch.status,
                    batch.total_count,
                    batch.valid_count,
                    batch.success_count,
                    batch.fail_count,
                    batch.skip_count,
                    batch.created_at,
                    batch.updated_at,
                    batch.finished_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn insert_row(&self, row: &MigrationRow) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_row(
                row_id,batch_id,row_no,source_key,source_hash,status,raw_json,normalized_json,
                error_code,error_message,id_med,id_med_unit,id_fac,id_med_pro,retry_count,updated_at
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"#,
                params![
                    row.row_id,
                    row.batch_id,
                    row.row_no,
                    row.source_key,
                    row.source_hash,
                    row.status,
                    Value::Object(row.raw_data.clone()).to_string(),
                    Value::Object(row.normalized_data.clone()).to_string(),
                    row.error_code,
                    row.error_message,
                    row.id_med,
                    row.id_med_unit,
                    row.id_fac,
                    row.id_med_pro,
                    row.retry_count,
                    row.updated_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn update_row_result(&self, row: &MigrationRow) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection.execute(
            r#"UPDATE migration_row SET status=?2,error_code=?3,error_message=?4,id_med=?5,
                id_med_unit=?6,id_fac=?7,id_med_pro=?8,retry_count=?9,updated_at=?10 WHERE row_id=?1"#,
            params![row.row_id,row.status,row.error_code,row.error_message,row.id_med,row.id_med_unit,
                row.id_fac,row.id_med_pro,row.retry_count,row.updated_at],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_batch_counts(
        &self,
        batch_id: &str,
        status: &str,
        valid: usize,
        success: usize,
        fail: usize,
        skip: usize,
        finished: bool,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let finished_at = if finished { Some(now.clone()) } else { None };
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection.execute(
            r#"UPDATE migration_batch SET status=?2,valid_count=?3,success_count=?4,fail_count=?5,
                skip_count=?6,updated_at=?7,finished_at=COALESCE(?8,finished_at) WHERE batch_id=?1"#,
            params![batch_id,status,valid,success,fail,skip,now,finished_at],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn insert_audit(&self, audit: &MigrationAudit) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection.execute(
            r#"INSERT INTO migration_audit(audit_id,batch_id,row_id,trace_id,operation,target_table,
                target_id,result,before_json,after_json,message,operator_id,operated_at)
                VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"#,
            params![audit.audit_id,audit.batch_id,audit.row_id,audit.trace_id,audit.operation,
                audit.target_table,audit.target_id,audit.result,audit.before_data.to_string(),
                audit.after_data.to_string(),audit.message,audit.operator_id,audit.operated_at],
        ).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn load_batch(&self, batch_id: &str) -> Result<BatchDetail, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let batch = connection.query_row(
            r#"SELECT batch_id,batch_name,source_type,source_name,source_description,conflict_strategy,
                allow_create_factory,idempotency_key,status,total_count,valid_count,success_count,
                fail_count,skip_count,created_at,updated_at,finished_at FROM migration_batch WHERE batch_id=?1"#,
            params![batch_id], map_batch,
        ).map_err(|error| error.to_string())?;

        let mut row_statement = connection.prepare(
            r#"SELECT row_id,batch_id,row_no,source_key,source_hash,status,raw_json,normalized_json,
                error_code,error_message,id_med,id_med_unit,id_fac,id_med_pro,retry_count,updated_at
                FROM migration_row WHERE batch_id=?1 ORDER BY row_no"#,
        ).map_err(|error| error.to_string())?;
        let rows = row_statement
            .query_map(params![batch_id], map_migration_row)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;

        let mut audit_statement = connection
            .prepare(
                r#"SELECT audit_id,batch_id,row_id,trace_id,operation,target_table,target_id,result,
                before_json,after_json,message,operator_id,operated_at FROM migration_audit
                WHERE batch_id=?1 ORDER BY operated_at DESC"#,
            )
            .map_err(|error| error.to_string())?;
        let audits = audit_statement
            .query_map(params![batch_id], map_audit)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(BatchDetail {
            batch,
            rows,
            audits,
        })
    }

    pub fn recent_batches(&self, limit: usize) -> Result<Vec<MigrationBatch>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let mut statement = connection.prepare(
            r#"SELECT batch_id,batch_name,source_type,source_name,source_description,conflict_strategy,
                allow_create_factory,idempotency_key,status,total_count,valid_count,success_count,
                fail_count,skip_count,created_at,updated_at,finished_at FROM migration_batch
                ORDER BY created_at DESC LIMIT ?1"#,
        ).map_err(|error| error.to_string())?;
        let batches = statement
            .query_map(params![limit], map_batch)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(batches)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn audit_event(
        &self,
        batch_id: &str,
        row_id: &str,
        operation: &str,
        table: &str,
        target_id: &str,
        result: &str,
        before: Value,
        after: Value,
        message: &str,
        operator_id: &str,
        trace_id: &str,
    ) -> Result<(), String> {
        self.insert_audit(&MigrationAudit {
            audit_id: new_object_id(),
            batch_id: batch_id.to_string(),
            row_id: row_id.to_string(),
            trace_id: trace_id.to_string(),
            operation: operation.to_string(),
            target_table: table.to_string(),
            target_id: target_id.to_string(),
            result: result.to_string(),
            before_data: before,
            after_data: after,
            message: message.to_string(),
            operator_id: operator_id.to_string(),
            operated_at: Utc::now().to_rfc3339(),
        })
    }
}

fn map_batch(row: &rusqlite::Row<'_>) -> rusqlite::Result<MigrationBatch> {
    Ok(MigrationBatch {
        batch_id: row.get(0)?,
        batch_name: row.get(1)?,
        source_type: row.get(2)?,
        source_name: row.get(3)?,
        source_description: row.get(4)?,
        conflict_strategy: row.get(5)?,
        allow_create_factory: row.get::<_, i32>(6)? == 1,
        idempotency_key: row.get(7)?,
        status: row.get(8)?,
        total_count: row.get(9)?,
        valid_count: row.get(10)?,
        success_count: row.get(11)?,
        fail_count: row.get(12)?,
        skip_count: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
        finished_at: row.get(16)?,
    })
}

fn map_migration_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MigrationRow> {
    let raw: String = row.get(6)?;
    let normalized: String = row.get(7)?;
    Ok(MigrationRow {
        row_id: row.get(0)?,
        batch_id: row.get(1)?,
        row_no: row.get(2)?,
        source_key: row.get(3)?,
        source_hash: row.get(4)?,
        status: row.get(5)?,
        raw_data: parse_map(&raw),
        normalized_data: parse_map(&normalized),
        error_code: row.get(8)?,
        error_message: row.get(9)?,
        id_med: row.get(10)?,
        id_med_unit: row.get(11)?,
        id_fac: row.get(12)?,
        id_med_pro: row.get(13)?,
        retry_count: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn map_audit(row: &rusqlite::Row<'_>) -> rusqlite::Result<MigrationAudit> {
    let before: String = row.get(8)?;
    let after: String = row.get(9)?;
    Ok(MigrationAudit {
        audit_id: row.get(0)?,
        batch_id: row.get(1)?,
        row_id: row.get(2)?,
        trace_id: row.get(3)?,
        operation: row.get(4)?,
        target_table: row.get(5)?,
        target_id: row.get(6)?,
        result: row.get(7)?,
        before_data: serde_json::from_str(&before).unwrap_or(Value::Null),
        after_data: serde_json::from_str(&after).unwrap_or(Value::Null),
        message: row.get(10)?,
        operator_id: row.get(11)?,
        operated_at: row.get(12)?,
    })
}

fn parse_map(json: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}
