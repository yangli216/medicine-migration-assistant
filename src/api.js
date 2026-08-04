import { invoke } from "@tauri-apps/api/core";
import { applyFieldRule } from "./transforms";

export const isDesktop =
  typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);

const mockBatches = new Map();

export async function command(name, args = {}) {
  if (isDesktop) return invoke(name, args);
  if (name === "app_health")
    return {
      ready: true,
      runtime: "browser-preview",
      keyRule: "bson-object-id-24-hex",
      supportedDirectDatabase: [
        "mysql",
        "oracle",
        "dameng",
        "opengauss",
        "kingbase",
        "postgresql",
      ],
    };
  if (name === "list_database_drivers") return ["Oracle 19 ODBC driver"];
  if (name === "list_driver_packs")
    return [
      mockDriverPack("mysql", "MySQL 原生驱动", "内置", "bundled", ""),
      mockDriverPack(
        "oracle",
        "Oracle Instant Client ODBC 19c",
        "19.16 · macOS Intel",
        "bundled",
        "Oracle 19 ODBC driver",
      ),
      mockDriverPack(
        "dameng",
        "达梦 DM8 通用 ODBC",
        "通用",
        "profile-ready",
        "DM8 ODBC DRIVER",
      ),
      mockDriverPack(
        "opengauss",
        "openGauss 通用 ODBC",
        "6.0 LTS",
        "profile-ready",
        "openGauss",
      ),
      mockDriverPack(
        "kingbase",
        "KingbaseES 通用 ODBC",
        "8.6",
        "profile-ready",
        "KingbaseES 8.6 ODBC Driver",
      ),
      mockDriverPack(
        "postgresql",
        "PostgreSQL Unicode ODBC",
        "17.x",
        "profile-ready",
        "PostgreSQL Unicode(x64)",
      ),
    ];
  if (name === "generate_object_id") return objectId();
  if (name === "test_database_connection") {
    if (
      args.profile?.kind &&
      !["mysql", "oracle"].includes(args.profile.kind)
    ) {
      throw new Error(
        "当前只有连接配置，未检测到本机 ODBC 驱动；请在桌面版安装或导入对应驱动后重试。",
      );
    }
    return {
      ok: true,
      databaseVersion:
        args.profile?.kind === "oracle" ? "Oracle（演示）" : "MySQL 8.0（演示）",
      latencyMs: 38,
      message: "浏览器预览模式：桌面版将执行真实连接测试",
    };
  }
  if (name === "inspect_target_schema")
    return {
      ok: true,
      checkedTables: [
        "hi_bd_med",
        "hi_bd_med_alias",
        "hi_bd_med_unit",
        "hi_bd_fac",
        "hi_bd_med_pro",
      ],
      warnings: ["浏览器预览模式不会验证真实写权限"],
      message: "目标药品表结构与当前迁移版本兼容",
    };
  if (name === "list_source_tables")
    return ["T_DRUG_INFO", "T_DRUG_PRICE", "T_FACTORY"];
  if (name === "preview_source") return demoPreview();
  if (name === "prepare_migration_batch") return mockPrepare(args.request);
  if (name === "execute_migration_batch") return mockExecute(args.request);
  if (name === "load_migration_batch") return mockBatches.get(args.batchId);
  if (name === "list_recent_batches")
    return [...mockBatches.values()].map((item) => item.batch);
  throw new Error(`浏览器预览未实现命令：${name}`);
}

function mockDriverPack(databaseKind, title, version, state, defaultDriver) {
  return {
    id: `${databaseKind}-preview`,
    databaseKind,
    title,
    version,
    defaultDriver,
    defaultPort: 0,
    delivery: state === "bundled" ? "bundled" : "open-source-pack",
    licenseNote: "驱动配置已预置，桌面版会检测本机实际安装状态。",
    officialUrl: "",
    state,
    detectedDriver: ["installed", "bundled"].includes(state)
      ? defaultDriver
      : null,
    platform: "preview",
    architecture: "x64",
  };
}

export function objectId() {
  const timestamp = Math.floor(Date.now() / 1000)
    .toString(16)
    .padStart(8, "0");
  const random = crypto.getRandomValues(new Uint8Array(8));
  return (
    timestamp +
    [...random].map((byte) => byte.toString(16).padStart(2, "0")).join("")
  );
}

function demoPreview() {
  const rows = demoRows();
  return {
    columns: Object.keys(rows[0]),
    rows,
    truncated: false,
    elapsedMs: 42,
  };
}

export function demoRows() {
  return [
    {
      DRUG_CODE: "6201030012345",
      DRUG_NAME: "阿莫西林胶囊",
      DRUG_TYPE: "1",
      CSTMG_ID: "66aa10244f0d4826ac110001",
      FORM_CODE: "CAP",
      PRE_UNIT: "粒",
      DOSE: "0.25",
      DOSE_UNIT: "g",
      SPEC: "0.25g/粒",
      USAGE_CODE: "PO",
      FREQ_CODE: "TID",
      FACTORY_NAME: "石药集团欧意药业有限公司",
      PRODUCT_NAME: "阿莫西林胶囊",
      SALE_UNIT: "盒",
      PACK_FACTOR: "24",
      BUY_PRICE: "12.80",
      RETAIL_PRICE: "15.60",
      APPROVAL_NO: "国药准字H13020728",
    },
    {
      DRUG_CODE: "6201040005678",
      DRUG_NAME: "氯化钠注射液",
      DRUG_TYPE: "1",
      CSTMG_ID: "66aa10244f0d4826ac110002",
      FORM_CODE: "INJ",
      PRE_UNIT: "瓶",
      DOSE: "100",
      DOSE_UNIT: "ml",
      SPEC: "100ml:0.9g/瓶",
      USAGE_CODE: "IVGTT",
      FREQ_CODE: "QD",
      FACTORY_NAME: "江苏恒瑞医药股份有限公司",
      PRODUCT_NAME: "氯化钠注射液",
      SALE_UNIT: "瓶",
      PACK_FACTOR: "1",
      BUY_PRICE: "3.20",
      RETAIL_PRICE: "4.10",
      APPROVAL_NO: "国药准字H32021600",
    },
    {
      DRUG_CODE: "6201050098765",
      DRUG_NAME: "奥美拉唑肠溶胶囊",
      DRUG_TYPE: "1",
      CSTMG_ID: "66aa10244f0d4826ac110003",
      FORM_CODE: "CAP",
      PRE_UNIT: "粒",
      DOSE: "20",
      DOSE_UNIT: "mg",
      SPEC: "20mg/粒",
      USAGE_CODE: "PO",
      FREQ_CODE: "QD",
      FACTORY_NAME: "阿斯利康制药有限公司",
      PRODUCT_NAME: "洛赛克",
      SALE_UNIT: "盒",
      PACK_FACTOR: "14",
      BUY_PRICE: "42.00",
      RETAIL_PRICE: "49.80",
      APPROVAL_NO: "国药准字H20030447",
    },
  ];
}

function mockPrepare(request) {
  const batchId = objectId();
  const now = new Date().toISOString();
  const rows = request.rows.map((raw, index) => {
    const normalized = {};
    request.mappings.forEach((mapping) => {
      normalized[mapping.targetField] = applyFieldRule(
        raw[mapping.sourceField],
        mapping,
      );
    });
    const errors = [];
    [
      "naMed",
      "sdMed",
      "idCstmg",
      "sdDose",
      "unitPre",
      "unitSale",
      "unitSaleFactor",
      "pricePur",
      "priceSale",
    ].forEach((key) => {
      if (
        normalized[key] === undefined ||
        normalized[key] === null ||
        `${normalized[key]}`.trim() === ""
      )
        errors.push(`${key}不能为空`);
    });
    return {
      rowId: objectId(),
      batchId,
      rowNo: index + 1,
      sourceKey: raw._sourceKey || `${index + 1}`,
      sourceHash: objectId(),
      status: errors.length ? "INVALID" : "VALIDATED",
      rawData: raw,
      normalizedData: normalized,
      errorCode: errors.length ? "VALIDATION_ERROR" : "",
      errorMessage: errors.join("；"),
      idMed: "",
      idMedUnit: "",
      idFac: "",
      idMedPro: "",
      retryCount: 0,
      updatedAt: now,
    };
  });
  const valid = rows.filter((row) => row.status === "VALIDATED").length;
  const batch = {
    batchId,
    batchName: request.batchName,
    sourceType: request.sourceType,
    sourceName: request.sourceName,
    sourceDescription: request.sourceDescription || "",
    conflictStrategy: request.conflictStrategy,
    allowCreateFactory: request.allowCreateFactory,
    idempotencyKey: request.idempotencyKey || "",
    status: valid ? "VALIDATED" : "INVALID",
    totalCount: rows.length,
    validCount: valid,
    successCount: 0,
    failCount: rows.length - valid,
    skipCount: 0,
    createdAt: now,
    updatedAt: now,
    finishedAt: null,
  };
  const detail = {
    batch,
    rows,
    audits: [
      {
        auditId: objectId(),
        batchId,
        rowId: "",
        traceId: objectId(),
        operation: "PREPARE",
        targetTable: "migration_batch",
        targetId: batchId,
        result: batch.status,
        beforeData: null,
        afterData: batch,
        message: `暂存并校验完成：有效${valid}行，无效${rows.length - valid}行`,
        operatorId: "demo",
        operatedAt: now,
      },
    ],
  };
  mockBatches.set(batchId, detail);
  return detail;
}

function mockExecute(request) {
  const detail = mockBatches.get(request.batchId);
  const now = new Date().toISOString();
  detail.rows = detail.rows.map((row) => {
    if (
      row.status !== "VALIDATED" &&
      !(request.failedOnly && row.status === "FAILED")
    )
      return row;
    const updated = {
      ...row,
      status: "SUCCESS",
      idMed: objectId(),
      idMedUnit: objectId(),
      idFac: objectId(),
      idMedPro: objectId(),
      updatedAt: now,
    };
    detail.audits.unshift({
      auditId: objectId(),
      batchId: request.batchId,
      rowId: row.rowId,
      traceId: objectId(),
      operation: "ROW_FINISH",
      targetTable: "hi_bd_med_pro",
      targetId: updated.idMedPro,
      result: "SUCCESS",
      beforeData: null,
      afterData: { idMed: updated.idMed, idMedPro: updated.idMedPro },
      message: "浏览器预览：模拟直接写表成功",
      operatorId: request.operatorId,
      operatedAt: now,
    });
    return updated;
  });
  detail.batch = {
    ...detail.batch,
    status: "SUCCESS",
    validCount: 0,
    successCount: detail.rows.filter((row) => row.status === "SUCCESS").length,
    failCount: detail.rows.filter((row) =>
      ["FAILED", "INVALID"].includes(row.status),
    ).length,
    updatedAt: now,
    finishedAt: now,
  };
  mockBatches.set(request.batchId, detail);
  return detail;
}
