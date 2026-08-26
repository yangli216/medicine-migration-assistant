use crate::id::new_object_id;
use crate::model::{BatchDetail, MigrationAudit, MigrationBatch, MigrationRow};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::Path;
use std::sync::Mutex;

pub struct LocalStore {
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLinkSnapshot {
    pub link_id: String,
    pub tenant_id: String,
    pub source_type: String,
    pub source_name: String,
    pub source_key: String,
    pub source_hash: String,
    pub batch_id: String,
    pub row_id: String,
    pub id_med: String,
    pub id_med_unit: String,
    pub id_fac: String,
    pub id_med_pro: String,
    pub write_manifest: Value,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryLocationMapping {
    pub source_location_key: String,
    pub source_kind: String,
    pub source_location_name: String,
    #[serde(default)]
    pub source_organization_id: String,
    #[serde(default)]
    pub resolved_source_location_key: String,
    pub target_id_sto: String,
    pub target_name: String,
    pub target_id_org: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryOrganizationMapping {
    pub source_organization_id: String,
    pub source_organization_name: String,
    pub target_organization_id: String,
    pub target_organization_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryMedicineMapping {
    pub source_product_key: String,
    pub target_identity: String,
    pub target_organization_id: String,
    pub id_med: String,
    pub id_med_pro: String,
    pub match_method: String,
    pub source_snapshot: Value,
    pub target_snapshot: Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryLinkSnapshot {
    pub tenant_id: String,
    pub source_name: String,
    pub source_stock_key: String,
    pub target_identity: String,
    pub source_hash: String,
    pub batch_id: String,
    pub row_id: String,
    pub id_sto: String,
    pub id_sto_med: String,
    pub id_sto_inv: String,
    pub id_inv_log: String,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl SourceLinkSnapshot {
    pub fn manages(&self, table: &str, target_id: &str) -> bool {
        self.write_manifest
            .as_array()
            .into_iter()
            .flatten()
            .any(|event| {
                matches!(
                    event.get("operation").and_then(Value::as_str),
                    Some("INSERT" | "UPDATE")
                ) && event.get("table").and_then(Value::as_str) == Some(table)
                    && event.get("targetId").and_then(Value::as_str) == Some(target_id)
            })
    }
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
                    conflict_strategy TEXT NOT NULL DEFAULT 'INCREMENTAL',
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
                CREATE TABLE IF NOT EXISTS migration_source_link (
                    link_id TEXT PRIMARY KEY,
                    tenant_id TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_key TEXT NOT NULL,
                    source_hash TEXT NOT NULL,
                    batch_id TEXT NOT NULL,
                    row_id TEXT NOT NULL,
                    id_med TEXT NOT NULL DEFAULT '',
                    id_med_unit TEXT NOT NULL DEFAULT '',
                    id_fac TEXT NOT NULL DEFAULT '',
                    id_med_pro TEXT NOT NULL DEFAULT '',
                    write_manifest_json TEXT NOT NULL DEFAULT '[]',
                    active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    UNIQUE(tenant_id,source_type,source_name,source_key)
                );
                CREATE INDEX IF NOT EXISTS ix_mig_link_batch ON migration_source_link(batch_id);
                CREATE TABLE IF NOT EXISTS migration_factory_link (
                    tenant_id TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_factory_key TEXT NOT NULL,
                    target_id_fac TEXT NOT NULL,
                    source_factory_name TEXT NOT NULL DEFAULT '',
                    batch_id TEXT NOT NULL DEFAULT '',
                    row_id TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY(tenant_id,source_type,source_name,source_factory_key)
                );
                CREATE INDEX IF NOT EXISTS ix_mig_factory_target
                    ON migration_factory_link(tenant_id,target_id_fac);
                CREATE TABLE IF NOT EXISTS migration_inventory_location_mapping (
                    tenant_id TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_location_key TEXT NOT NULL,
                    target_identity TEXT NOT NULL,
                    source_kind TEXT NOT NULL DEFAULT '',
                    source_location_name TEXT NOT NULL DEFAULT '',
                    source_organization_id TEXT NOT NULL DEFAULT '',
                    resolved_source_location_key TEXT NOT NULL DEFAULT '',
                    target_id_sto TEXT NOT NULL,
                    target_name TEXT NOT NULL DEFAULT '',
                    target_id_org TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY(tenant_id,source_name,source_location_key,target_identity)
                );
                CREATE INDEX IF NOT EXISTS ix_mig_inv_location_target
                    ON migration_inventory_location_mapping(tenant_id,target_identity,target_id_sto);
                CREATE TABLE IF NOT EXISTS migration_inventory_organization_mapping (
                    tenant_id TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_organization_id TEXT NOT NULL,
                    target_organization_id TEXT NOT NULL,
                    source_organization_name TEXT NOT NULL DEFAULT '',
                    target_organization_name TEXT NOT NULL DEFAULT '',
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY(tenant_id,source_name,source_organization_id)
                );
                CREATE TABLE IF NOT EXISTS migration_inventory_medicine_mapping (
                    tenant_id TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_product_key TEXT NOT NULL,
                    target_identity TEXT NOT NULL,
                    target_organization_id TEXT NOT NULL DEFAULT '',
                    id_med TEXT NOT NULL,
                    id_med_pro TEXT NOT NULL,
                    match_method TEXT NOT NULL DEFAULT 'MANUAL',
                    source_snapshot_json TEXT NOT NULL DEFAULT '{}',
                    target_snapshot_json TEXT NOT NULL DEFAULT '{}',
                    active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY(
                        tenant_id,source_name,source_product_key,target_identity,target_organization_id
                    )
                );
                CREATE INDEX IF NOT EXISTS ix_mig_inv_med_target
                    ON migration_inventory_medicine_mapping(
                        tenant_id,target_identity,target_organization_id,id_med_pro
                    );
                CREATE TABLE IF NOT EXISTS migration_inventory_link (
                    tenant_id TEXT NOT NULL,
                    source_name TEXT NOT NULL,
                    source_stock_key TEXT NOT NULL,
                    target_identity TEXT NOT NULL,
                    source_hash TEXT NOT NULL,
                    batch_id TEXT NOT NULL,
                    row_id TEXT NOT NULL,
                    id_sto TEXT NOT NULL,
                    id_sto_med TEXT NOT NULL,
                    id_sto_inv TEXT NOT NULL,
                    id_inv_log TEXT NOT NULL,
                    active INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY(tenant_id,source_name,source_stock_key,target_identity)
                );
                CREATE INDEX IF NOT EXISTS ix_mig_inv_link_batch
                    ON migration_inventory_link(batch_id);
                CREATE TABLE IF NOT EXISTS app_setting (
                    setting_key TEXT PRIMARY KEY,
                    value_json TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                PRAGMA optimize;
                "#,
            )
            .map_err(|error| error.to_string())?;
        ensure_column(
            &connection,
            "migration_inventory_location_mapping",
            "source_organization_id",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(
            &connection,
            "migration_inventory_location_mapping",
            "resolved_source_location_key",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn load_setting(&self, key: &str) -> Result<Option<Value>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let value = connection
            .query_row(
                "SELECT value_json FROM app_setting WHERE setting_key=?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        value
            .map(|json| serde_json::from_str(&json).map_err(|error| error.to_string()))
            .transpose()
    }

    pub fn save_setting(&self, key: &str, value: &Value) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO app_setting(setting_key,value_json,updated_at)
                   VALUES(?1,?2,?3)
                   ON CONFLICT(setting_key) DO UPDATE SET
                     value_json=excluded.value_json,
                     updated_at=excluded.updated_at"#,
                params![key, value.to_string(), Utc::now().to_rfc3339()],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn delete_setting(&self, key: &str) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute("DELETE FROM app_setting WHERE setting_key=?1", params![key])
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn save_inventory_location_mappings(
        &self,
        tenant_id: &str,
        source_name: &str,
        target_identity: &str,
        mappings: &[InventoryLocationMapping],
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        for mapping in mappings {
            transaction
                .execute(
                    r#"INSERT INTO migration_inventory_location_mapping(
                       tenant_id,source_name,source_location_key,target_identity,source_kind,
                       source_location_name,source_organization_id,resolved_source_location_key,
                       target_id_sto,target_name,target_id_org,updated_at
                    ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
                    ON CONFLICT(tenant_id,source_name,source_location_key,target_identity) DO UPDATE SET
                       source_kind=excluded.source_kind,source_location_name=excluded.source_location_name,
                       source_organization_id=excluded.source_organization_id,
                       resolved_source_location_key=excluded.resolved_source_location_key,
                       target_id_sto=excluded.target_id_sto,target_name=excluded.target_name,
                       target_id_org=excluded.target_id_org,updated_at=excluded.updated_at"#,
                    params![
                        tenant_id,
                        source_name,
                        mapping.source_location_key,
                        target_identity,
                        mapping.source_kind,
                        mapping.source_location_name,
                        mapping.source_organization_id,
                        mapping.resolved_source_location_key,
                        mapping.target_id_sto,
                        mapping.target_name,
                        mapping.target_id_org,
                        now
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn save_inventory_organization_mappings(
        &self,
        tenant_id: &str,
        source_name: &str,
        mappings: &[InventoryOrganizationMapping],
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        for mapping in mappings {
            transaction
                .execute(
                    r#"INSERT INTO migration_inventory_organization_mapping(
                       tenant_id,source_name,source_organization_id,target_organization_id,
                       source_organization_name,target_organization_name,updated_at
                    ) VALUES(?1,?2,?3,?4,?5,?6,?7)
                    ON CONFLICT(tenant_id,source_name,source_organization_id) DO UPDATE SET
                       target_organization_id=excluded.target_organization_id,
                       source_organization_name=excluded.source_organization_name,
                       target_organization_name=excluded.target_organization_name,
                       updated_at=excluded.updated_at"#,
                    params![
                        tenant_id,
                        source_name,
                        mapping.source_organization_id,
                        mapping.target_organization_id,
                        mapping.source_organization_name,
                        mapping.target_organization_name,
                        now
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn load_inventory_organization_mappings(
        &self,
        tenant_id: &str,
        source_name: &str,
    ) -> Result<Vec<InventoryOrganizationMapping>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let mut statement = connection
            .prepare(
                r#"SELECT source_organization_id,source_organization_name,
                   target_organization_id,target_organization_name
                   FROM migration_inventory_organization_mapping
                   WHERE tenant_id=?1 AND source_name=?2 ORDER BY source_organization_id"#,
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![tenant_id, source_name], |row| {
                Ok(InventoryOrganizationMapping {
                    source_organization_id: row.get(0)?,
                    source_organization_name: row.get(1)?,
                    target_organization_id: row.get(2)?,
                    target_organization_name: row.get(3)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn load_inventory_location_mappings(
        &self,
        tenant_id: &str,
        source_name: &str,
        target_identity: &str,
    ) -> Result<Vec<InventoryLocationMapping>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let mut statement = connection
            .prepare(
                r#"SELECT source_location_key,source_kind,source_location_name,
                   source_organization_id,resolved_source_location_key,target_id_sto,
                   target_name,target_id_org FROM migration_inventory_location_mapping
                   WHERE tenant_id=?1 AND source_name=?2 AND target_identity=?3
                   ORDER BY source_location_key"#,
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![tenant_id, source_name, target_identity], |row| {
                Ok(InventoryLocationMapping {
                    source_location_key: row.get(0)?,
                    source_kind: row.get(1)?,
                    source_location_name: row.get(2)?,
                    source_organization_id: row.get(3)?,
                    resolved_source_location_key: row.get(4)?,
                    target_id_sto: row.get(5)?,
                    target_name: row.get(6)?,
                    target_id_org: row.get(7)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn find_inventory_medicine_mapping(
        &self,
        tenant_id: &str,
        source_name: &str,
        source_product_key: &str,
        target_identity: &str,
        target_organization_id: &str,
    ) -> Result<Option<InventoryMedicineMapping>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .query_row(
                r#"SELECT source_product_key,target_identity,target_organization_id,
                   id_med,id_med_pro,match_method,source_snapshot_json,target_snapshot_json,
                   updated_at FROM migration_inventory_medicine_mapping
                   WHERE tenant_id=?1 AND source_name=?2 AND source_product_key=?3
                     AND target_identity=?4 AND target_organization_id=?5 AND active=1"#,
                params![
                    tenant_id,
                    source_name,
                    source_product_key,
                    target_identity,
                    target_organization_id
                ],
                |row| {
                    let source_snapshot: String = row.get(6)?;
                    let target_snapshot: String = row.get(7)?;
                    Ok(InventoryMedicineMapping {
                        source_product_key: row.get(0)?,
                        target_identity: row.get(1)?,
                        target_organization_id: row.get(2)?,
                        id_med: row.get(3)?,
                        id_med_pro: row.get(4)?,
                        match_method: row.get(5)?,
                        source_snapshot: serde_json::from_str(&source_snapshot)
                            .unwrap_or(Value::Null),
                        target_snapshot: serde_json::from_str(&target_snapshot)
                            .unwrap_or(Value::Null),
                        updated_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_inventory_medicine_mapping(
        &self,
        tenant_id: &str,
        source_name: &str,
        source_product_key: &str,
        target_identity: &str,
        target_organization_id: &str,
        id_med: &str,
        id_med_pro: &str,
        match_method: &str,
        source_snapshot: &Value,
        target_snapshot: &Value,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_inventory_medicine_mapping(
                   tenant_id,source_name,source_product_key,target_identity,target_organization_id,
                   id_med,id_med_pro,match_method,source_snapshot_json,target_snapshot_json,
                   active,created_at,updated_at
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,1,?11,?11)
                ON CONFLICT(
                    tenant_id,source_name,source_product_key,target_identity,target_organization_id
                ) DO UPDATE SET
                   id_med=excluded.id_med,id_med_pro=excluded.id_med_pro,
                   match_method=excluded.match_method,
                   source_snapshot_json=excluded.source_snapshot_json,
                   target_snapshot_json=excluded.target_snapshot_json,
                   active=1,updated_at=excluded.updated_at"#,
                params![
                    tenant_id,
                    source_name,
                    source_product_key,
                    target_identity,
                    target_organization_id,
                    id_med,
                    id_med_pro,
                    match_method,
                    source_snapshot.to_string(),
                    target_snapshot.to_string(),
                    now
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn find_inventory_link(
        &self,
        tenant_id: &str,
        source_name: &str,
        source_stock_key: &str,
        target_identity: &str,
    ) -> Result<Option<InventoryLinkSnapshot>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .query_row(
                r#"SELECT tenant_id,source_name,source_stock_key,target_identity,source_hash,
                   batch_id,row_id,id_sto,id_sto_med,id_sto_inv,id_inv_log,active,created_at,updated_at
                   FROM migration_inventory_link
                   WHERE tenant_id=?1 AND source_name=?2 AND source_stock_key=?3
                     AND target_identity=?4 AND active=1"#,
                params![tenant_id, source_name, source_stock_key, target_identity],
                |row| {
                    Ok(InventoryLinkSnapshot {
                        tenant_id: row.get(0)?,
                        source_name: row.get(1)?,
                        source_stock_key: row.get(2)?,
                        target_identity: row.get(3)?,
                        source_hash: row.get(4)?,
                        batch_id: row.get(5)?,
                        row_id: row.get(6)?,
                        id_sto: row.get(7)?,
                        id_sto_med: row.get(8)?,
                        id_sto_inv: row.get(9)?,
                        id_inv_log: row.get(10)?,
                        active: row.get::<_, i32>(11)? != 0,
                        created_at: row.get(12)?,
                        updated_at: row.get(13)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(dead_code)]
    pub fn record_inventory_link(
        &self,
        tenant_id: &str,
        source_name: &str,
        source_stock_key: &str,
        target_identity: &str,
        source_hash: &str,
        batch_id: &str,
        row_id: &str,
        id_sto: &str,
        id_sto_med: &str,
        id_sto_inv: &str,
        id_inv_log: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_inventory_link(
                   tenant_id,source_name,source_stock_key,target_identity,source_hash,batch_id,row_id,
                   id_sto,id_sto_med,id_sto_inv,id_inv_log,active,created_at,updated_at
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,1,?12,?12)
                ON CONFLICT(tenant_id,source_name,source_stock_key,target_identity) DO UPDATE SET
                   source_hash=excluded.source_hash,batch_id=excluded.batch_id,row_id=excluded.row_id,
                   id_sto=excluded.id_sto,id_sto_med=excluded.id_sto_med,
                   id_sto_inv=excluded.id_sto_inv,id_inv_log=excluded.id_inv_log,
                   active=1,updated_at=excluded.updated_at"#,
                params![
                    tenant_id,
                    source_name,
                    source_stock_key,
                    target_identity,
                    source_hash,
                    batch_id,
                    row_id,
                    id_sto,
                    id_sto_med,
                    id_sto_inv,
                    id_inv_log,
                    now
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn deactivate_inventory_links_for_batch(&self, batch_id: &str) -> Result<usize, String> {
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                "UPDATE migration_inventory_link SET active=0,updated_at=?2 WHERE batch_id=?1 AND active=1",
                params![batch_id, now],
            )
            .map_err(|error| error.to_string())
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

    pub fn find_factory_link(
        &self,
        tenant_id: &str,
        source_type: &str,
        source_name: &str,
        source_factory_key: &str,
    ) -> Result<Option<String>, String> {
        if source_factory_key.trim().is_empty() {
            return Ok(None);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .query_row(
                r#"SELECT target_id_fac FROM migration_factory_link
                   WHERE tenant_id=?1 AND source_type=?2 AND source_name=?3
                     AND source_factory_key=?4"#,
                params![tenant_id, source_type, source_name, source_factory_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_factory_link_upsert(
        &self,
        tenant_id: &str,
        source_type: &str,
        source_name: &str,
        source_factory_key: &str,
        target_id_fac: &str,
        source_factory_name: &str,
        batch_id: &str,
        row_id: &str,
    ) -> Result<(), String> {
        if source_factory_key.trim().is_empty() || target_id_fac.trim().is_empty() {
            return Ok(());
        }
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_factory_link(
                    tenant_id,source_type,source_name,source_factory_key,target_id_fac,
                    source_factory_name,batch_id,row_id,created_at,updated_at
                ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)
                ON CONFLICT(tenant_id,source_type,source_name,source_factory_key) DO UPDATE SET
                    target_id_fac=excluded.target_id_fac,
                    source_factory_name=excluded.source_factory_name,
                    batch_id=excluded.batch_id,row_id=excluded.row_id,
                    updated_at=excluded.updated_at"#,
                params![
                    tenant_id,
                    source_type,
                    source_name,
                    source_factory_key,
                    target_id_fac,
                    source_factory_name,
                    batch_id,
                    row_id,
                    now
                ],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    pub fn find_source_link(
        &self,
        tenant_id: &str,
        source_type: &str,
        source_name: &str,
        source_key: &str,
    ) -> Result<Option<SourceLinkSnapshot>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .query_row(
                r#"SELECT link_id,tenant_id,source_type,source_name,source_key,source_hash,
                   batch_id,row_id,id_med,id_med_unit,id_fac,id_med_pro,write_manifest_json,
                   active,created_at,updated_at FROM migration_source_link
                   WHERE tenant_id=?1 AND source_type=?2 AND source_name=?3 AND source_key=?4
                     AND active=1"#,
                params![tenant_id, source_type, source_name, source_key],
                |row| {
                    let manifest: String = row.get(12)?;
                    Ok(SourceLinkSnapshot {
                        link_id: row.get(0)?,
                        tenant_id: row.get(1)?,
                        source_type: row.get(2)?,
                        source_name: row.get(3)?,
                        source_key: row.get(4)?,
                        source_hash: row.get(5)?,
                        batch_id: row.get(6)?,
                        row_id: row.get(7)?,
                        id_med: row.get(8)?,
                        id_med_unit: row.get(9)?,
                        id_fac: row.get(10)?,
                        id_med_pro: row.get(11)?,
                        write_manifest: serde_json::from_str(&manifest).unwrap_or(Value::Null),
                        active: row.get::<_, i32>(13)? != 0,
                        created_at: row.get(14)?,
                        updated_at: row.get(15)?,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())
    }

    pub fn restore_source_link(&self, snapshot: &SourceLinkSnapshot) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_source_link(
                    link_id,tenant_id,source_type,source_name,source_key,source_hash,batch_id,row_id,
                    id_med,id_med_unit,id_fac,id_med_pro,write_manifest_json,active,created_at,updated_at
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
                ON CONFLICT(tenant_id,source_type,source_name,source_key) DO UPDATE SET
                    link_id=excluded.link_id,source_hash=excluded.source_hash,
                    batch_id=excluded.batch_id,row_id=excluded.row_id,id_med=excluded.id_med,
                    id_med_unit=excluded.id_med_unit,id_fac=excluded.id_fac,
                    id_med_pro=excluded.id_med_pro,write_manifest_json=excluded.write_manifest_json,
                    active=excluded.active,created_at=excluded.created_at,updated_at=excluded.updated_at"#,
                params![
                    snapshot.link_id,
                    snapshot.tenant_id,
                    snapshot.source_type,
                    snapshot.source_name,
                    snapshot.source_key,
                    snapshot.source_hash,
                    snapshot.batch_id,
                    snapshot.row_id,
                    snapshot.id_med,
                    snapshot.id_med_unit,
                    snapshot.id_fac,
                    snapshot.id_med_pro,
                    snapshot.write_manifest.to_string(),
                    snapshot.active as i32,
                    snapshot.created_at,
                    snapshot.updated_at
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn upsert_source_link(
        &self,
        tenant_id: &str,
        source_type: &str,
        source_name: &str,
        row: &MigrationRow,
        write_manifest: Value,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                r#"INSERT INTO migration_source_link(
                    link_id,tenant_id,source_type,source_name,source_key,source_hash,batch_id,row_id,
                    id_med,id_med_unit,id_fac,id_med_pro,write_manifest_json,active,created_at,updated_at
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,1,?14,?14)
                ON CONFLICT(tenant_id,source_type,source_name,source_key) DO UPDATE SET
                    source_hash=excluded.source_hash,batch_id=excluded.batch_id,row_id=excluded.row_id,
                    id_med=excluded.id_med,id_med_unit=excluded.id_med_unit,id_fac=excluded.id_fac,
                    id_med_pro=excluded.id_med_pro,write_manifest_json=excluded.write_manifest_json,
                    active=1,updated_at=excluded.updated_at"#,
                params![
                    new_object_id(),
                    tenant_id,
                    source_type,
                    source_name,
                    row.source_key,
                    row.source_hash,
                    row.batch_id,
                    row.row_id,
                    row.id_med,
                    row.id_med_unit,
                    row.id_fac,
                    row.id_med_pro,
                    write_manifest.to_string(),
                    now
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_source_link_upsert(
        &self,
        tenant_id: &str,
        source_type: &str,
        source_name: &str,
        row: &MigrationRow,
        write_manifest: Value,
        operator_id: &str,
        trace_id: &str,
    ) -> Result<(), String> {
        let previous =
            self.find_source_link(tenant_id, source_type, source_name, &row.source_key)?;
        let before = previous
            .map(serde_json::to_value)
            .transpose()
            .map_err(|error| error.to_string())?
            .unwrap_or(Value::Null);
        self.upsert_source_link(
            tenant_id,
            source_type,
            source_name,
            row,
            write_manifest.clone(),
        )?;
        self.audit_event(
            &row.batch_id,
            &row.row_id,
            "SOURCE_LINK_UPSERT",
            "migration_source_link",
            &row.source_key,
            "SUCCESS",
            before,
            serde_json::json!({
                "tenantId":tenant_id,
                "sourceType":source_type,
                "sourceName":source_name,
                "sourceKey":row.source_key,
                "sourceHash":row.source_hash,
                "batchId":row.batch_id,
                "rowId":row.row_id,
                "idMed":row.id_med,
                "idMedUnit":row.id_med_unit,
                "idFac":row.id_fac,
                "idMedPro":row.id_med_pro,
                "writeManifest":write_manifest
            }),
            "已更新来源主键与目标主键迁移台账",
            operator_id,
            trace_id,
        )
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

    pub fn insert_prepared_batch(
        &self,
        batch: &MigrationBatch,
        mapping_json: &str,
        rows: &[MigrationRow],
    ) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
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
        {
            let mut statement = transaction
                .prepare_cached(
                    r#"INSERT INTO migration_row(
                    row_id,batch_id,row_no,source_key,source_hash,status,raw_json,normalized_json,
                    error_code,error_message,id_med,id_med_unit,id_fac,id_med_pro,retry_count,updated_at
                ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"#,
                )
                .map_err(|error| error.to_string())?;
            for row in rows {
                statement
                    .execute(params![
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
                    ])
                    .map_err(|error| error.to_string())?;
            }
        }
        transaction.commit().map_err(|error| error.to_string())
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

    pub fn commit_inventory_exception_confirmation(
        &self,
        row: &MigrationRow,
        batch: &MigrationBatch,
        audits: &[MigrationAudit],
    ) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                r#"UPDATE migration_row SET status=?2,error_code=?3,error_message=?4,id_med=?5,
                    id_med_unit=?6,id_fac=?7,id_med_pro=?8,retry_count=?9,updated_at=?10 WHERE row_id=?1"#,
                params![row.row_id,row.status,row.error_code,row.error_message,row.id_med,row.id_med_unit,
                    row.id_fac,row.id_med_pro,row.retry_count,row.updated_at],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                r#"UPDATE migration_batch SET status=?2,valid_count=?3,success_count=?4,
                    fail_count=?5,skip_count=?6,updated_at=?7,finished_at=NULL WHERE batch_id=?1"#,
                params![
                    batch.batch_id,
                    batch.status,
                    batch.valid_count,
                    batch.success_count,
                    batch.fail_count,
                    batch.skip_count,
                    batch.updated_at
                ],
            )
            .map_err(|error| error.to_string())?;
        for audit in audits {
            transaction
                .execute(
                    r#"INSERT INTO migration_audit(audit_id,batch_id,row_id,trace_id,operation,target_table,
                        target_id,result,before_json,after_json,message,operator_id,operated_at)
                        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"#,
                    params![audit.audit_id,audit.batch_id,audit.row_id,audit.trace_id,audit.operation,
                        audit.target_table,audit.target_id,audit.result,audit.before_data.to_string(),
                        audit.after_data.to_string(),audit.message,audit.operator_id,audit.operated_at],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())
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

    pub fn update_batch_status(&self, batch_id: &str, status: &str) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                "UPDATE migration_batch SET status=?2,updated_at=?3,finished_at=?3 WHERE batch_id=?1",
                params![batch_id, status, now],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn deactivate_source_link_for_row(
        &self,
        batch_id: &str,
        row_id: &str,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        connection
            .execute(
                "UPDATE migration_source_link SET active=0,updated_at=?3 WHERE batch_id=?1 AND row_id=?2",
                params![batch_id, row_id, now],
            )
            .map_err(|error| error.to_string())?;
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

    pub fn load_batch_mapping(&self, batch_id: &str) -> Result<Value, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "本地迁移库已锁定".to_string())?;
        let mapping = connection
            .query_row(
                "SELECT mapping_json FROM migration_batch WHERE batch_id=?1",
                params![batch_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| error.to_string())?;
        serde_json::from_str(&mapping).map_err(|error| error.to_string())
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

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| error.to_string())?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    if columns
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(column))
    {
        return Ok(());
    }
    connection
        .execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        InventoryLocationMapping, InventoryOrganizationMapping, LocalStore, SourceLinkSnapshot,
    };
    use crate::model::{MigrationBatch, MigrationRow};
    use serde_json::{json, Map};
    use std::path::Path;

    #[test]
    fn source_link_only_treats_insert_and_continuous_update_as_tool_managed() {
        let mut link = SourceLinkSnapshot {
            link_id: "link".into(),
            tenant_id: "tenant".into(),
            source_type: "PHIS27".into(),
            source_name: "source".into(),
            source_key: "1".into(),
            source_hash: "hash".into(),
            batch_id: "batch".into(),
            row_id: "row".into(),
            id_med: "med".into(),
            id_med_unit: "unit".into(),
            id_fac: "fac".into(),
            id_med_pro: "product".into(),
            write_manifest: json!([
                {"operation":"INSERT","table":"hi_bd_med","targetId":"med"},
                {"operation":"REUSE","table":"hi_bd_fac","targetId":"fac"}
            ]),
            active: true,
            created_at: "now".into(),
            updated_at: "now".into(),
        };
        assert!(link.manages("hi_bd_med", "med"));
        assert!(!link.manages("hi_bd_fac", "fac"));
        link.write_manifest = json!([{"operation":"UPDATE","table":"hi_bd_med","targetId":"med"}]);
        assert!(link.manages("hi_bd_med", "med"));
    }

    #[test]
    fn legacy_factory_key_reuses_the_same_target_factory() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        store
            .record_factory_link_upsert(
                "tenant",
                "PHIS27",
                "legacy-instance",
                "2001",
                "66aa10244f0d4826ac110001",
                "测试制药有限公司",
                "batch-1",
                "row-1",
            )
            .unwrap();
        assert_eq!(
            store
                .find_factory_link("tenant", "PHIS27", "legacy-instance", "2001")
                .unwrap()
                .as_deref(),
            Some("66aa10244f0d4826ac110001")
        );
        assert!(store
            .find_factory_link("tenant", "PHIS27", "another-instance", "2001")
            .unwrap()
            .is_none());
    }

    #[test]
    fn composite_product_keys_map_one_medicine_to_distinct_products() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let batch = MigrationBatch {
            batch_id: "batch".into(),
            batch_name: "二系列phis".into(),
            source_type: "PHIS27".into(),
            source_name: "二系列phis · source".into(),
            source_description: String::new(),
            conflict_strategy: "INCREMENTAL".into(),
            allow_create_factory: true,
            idempotency_key: "batch-key".into(),
            status: "SUCCESS".into(),
            total_count: 2,
            valid_count: 0,
            success_count: 2,
            fail_count: 0,
            skip_count: 0,
            created_at: "now".into(),
            updated_at: "now".into(),
            finished_at: Some("now".into()),
        };
        store.insert_batch(&batch, "{}").unwrap();
        for (row_id, source_key, id_fac, id_med_pro) in [
            ("row-1", "1001:2001", "fac-1", "product-1"),
            ("row-2", "1001:2002", "fac-2", "product-2"),
        ] {
            let row = MigrationRow {
                row_id: row_id.into(),
                batch_id: batch.batch_id.clone(),
                row_no: if row_id == "row-1" { 1 } else { 2 },
                source_key: source_key.into(),
                source_hash: format!("hash-{row_id}"),
                status: "SUCCESS".into(),
                raw_data: Map::new(),
                normalized_data: Map::new(),
                error_code: String::new(),
                error_message: String::new(),
                id_med: "shared-med".into(),
                id_med_unit: "shared-unit".into(),
                id_fac: id_fac.into(),
                id_med_pro: id_med_pro.into(),
                retry_count: 0,
                updated_at: "now".into(),
            };
            store.insert_row(&row).unwrap();
            store
                .record_source_link_upsert(
                    "tenant",
                    "PHIS27",
                    &batch.source_name,
                    &row,
                    json!([
                        {"operation":"REUSE","table":"hi_bd_med","targetId":"shared-med"},
                        {"operation":"INSERT","table":"hi_bd_med_pro","targetId":id_med_pro}
                    ]),
                    "operator",
                    "trace",
                )
                .unwrap();
        }
        let first = store
            .find_source_link("tenant", "PHIS27", &batch.source_name, "1001:2001")
            .unwrap()
            .unwrap();
        let second = store
            .find_source_link("tenant", "PHIS27", &batch.source_name, "1001:2002")
            .unwrap()
            .unwrap();
        assert_eq!(first.id_med, second.id_med);
        assert_ne!(first.id_fac, second.id_fac);
        assert_ne!(first.id_med_pro, second.id_med_pro);
    }

    #[test]
    fn merged_legacy_medicines_keep_distinct_source_links_to_one_target_medicine() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let batch = MigrationBatch {
            batch_id: "merge-batch".into(),
            batch_name: "二系列phis自动合并".into(),
            source_type: "PHIS27".into(),
            source_name: "二系列phis · source".into(),
            source_description: String::new(),
            conflict_strategy: "INCREMENTAL".into(),
            allow_create_factory: true,
            idempotency_key: "merge-batch-key".into(),
            status: "SUCCESS".into(),
            total_count: 2,
            valid_count: 0,
            success_count: 2,
            fail_count: 0,
            skip_count: 0,
            created_at: "now".into(),
            updated_at: "now".into(),
            finished_at: Some("now".into()),
        };
        store.insert_batch(&batch, "{}").unwrap();
        for (row_no, source_key, product_id) in
            [(1, "1001:2001", "product-1"), (2, "1002:2002", "product-2")]
        {
            let row = MigrationRow {
                row_id: format!("merge-row-{row_no}"),
                batch_id: batch.batch_id.clone(),
                row_no,
                source_key: source_key.into(),
                source_hash: format!("merge-hash-{row_no}"),
                status: "SUCCESS".into(),
                raw_data: Map::new(),
                normalized_data: Map::new(),
                error_code: String::new(),
                error_message: String::new(),
                id_med: "shared-med".into(),
                id_med_unit: "shared-unit".into(),
                id_fac: format!("fac-{row_no}"),
                id_med_pro: product_id.into(),
                retry_count: 0,
                updated_at: "now".into(),
            };
            store.insert_row(&row).unwrap();
            store
                .record_source_link_upsert(
                    "tenant",
                    "PHIS27",
                    &batch.source_name,
                    &row,
                    json!([{"operation":"REUSE","table":"hi_bd_med","targetId":"shared-med"}]),
                    "operator",
                    "trace",
                )
                .unwrap();
        }
        let first = store
            .find_source_link("tenant", "PHIS27", &batch.source_name, "1001:2001")
            .unwrap()
            .unwrap();
        let second = store
            .find_source_link("tenant", "PHIS27", &batch.source_name, "1002:2002")
            .unwrap()
            .unwrap();
        assert_eq!(first.id_med, "shared-med");
        assert_eq!(first.id_med, second.id_med);
        assert_ne!(first.source_key, second.source_key);
        assert_ne!(first.id_med_pro, second.id_med_pro);
    }

    #[test]
    fn inventory_location_mapping_is_isolated_by_target_database_identity() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let mapping = InventoryLocationMapping {
            source_location_key: "YK:420100001".into(),
            source_kind: "WAREHOUSE".into(),
            source_location_name: "中心药库".into(),
            source_organization_id: "420100001".into(),
            resolved_source_location_key: "YK:1".into(),
            target_id_sto: "target-storage".into(),
            target_name: "新系统中心药库".into(),
            target_id_org: "target-org".into(),
        };
        store
            .save_inventory_location_mappings(
                "tenant",
                "legacy-instance",
                "target-a",
                std::slice::from_ref(&mapping),
            )
            .unwrap();
        assert_eq!(
            store
                .load_inventory_location_mappings("tenant", "legacy-instance", "target-a")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store
                .load_inventory_location_mappings("tenant", "legacy-instance", "target-a")
                .unwrap()[0]
                .resolved_source_location_key,
            "YK:1"
        );
        assert!(store
            .load_inventory_location_mappings("tenant", "legacy-instance", "target-b")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn inventory_medicine_mapping_is_scoped_by_target_database_and_organization() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        store
            .save_inventory_medicine_mapping(
                "tenant",
                "legacy-instance",
                "101:4003",
                "target-a",
                "org-a",
                "med-a",
                "product-a",
                "MANUAL",
                &json!({"drugName":"多巴胺注射液"}),
                &json!({"idMedPro":"product-a"}),
            )
            .unwrap();
        let saved = store
            .find_inventory_medicine_mapping(
                "tenant",
                "legacy-instance",
                "101:4003",
                "target-a",
                "org-a",
            )
            .unwrap()
            .unwrap();
        assert_eq!(saved.id_med, "med-a");
        assert_eq!(saved.id_med_pro, "product-a");
        assert!(store
            .find_inventory_medicine_mapping(
                "tenant",
                "legacy-instance",
                "101:4003",
                "target-b",
                "org-a",
            )
            .unwrap()
            .is_none());
        assert!(store
            .find_inventory_medicine_mapping(
                "tenant",
                "legacy-instance",
                "101:4003",
                "target-a",
                "org-b",
            )
            .unwrap()
            .is_none());
    }

    #[test]
    fn inventory_organization_mapping_is_restored_for_the_same_legacy_database() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let mapping = InventoryOrganizationMapping {
            source_organization_id: "330108001".into(),
            source_organization_name: "老系统医院".into(),
            target_organization_id: "5b7e12988ddd9446d42a03c8".into(),
            target_organization_name: "新系统医院".into(),
        };
        store
            .save_inventory_organization_mappings(
                "tenant",
                "legacy-instance",
                std::slice::from_ref(&mapping),
            )
            .unwrap();
        assert_eq!(
            store
                .load_inventory_organization_mappings("tenant", "legacy-instance")
                .unwrap()[0]
                .target_organization_id,
            mapping.target_organization_id
        );
        assert!(store
            .load_inventory_organization_mappings("tenant", "another-instance")
            .unwrap()
            .is_empty());
    }
}
