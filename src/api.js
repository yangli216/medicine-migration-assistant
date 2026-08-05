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
  if (name === "probe_target_system") {
    const baseUrl = `${args.request?.baseUrl || ""}`.trim().replace(/\/+$/, "");
    if (!/^https?:\/\/[^/]+/i.test(baseUrl))
      throw new Error("请输入有效的新系统访问地址");
    return {
      ok: true,
      baseUrl,
      statusCode: 200,
      latencyMs: 35,
      message: "浏览器预览模式：新系统地址验证通过",
    };
  }
  if (name === "login_target_system") {
    const request = args.request || {};
    if (request.loginName !== "system")
      throw new Error("本功能仅允许 system（租户管理员）登录");
    if (!`${request.tenantId || ""}`.trim()) throw new Error("请输入租户编码");
    if (!request.password) throw new Error("请输入 system 密码");
    return {
      ok: true,
      baseUrl: `${request.baseUrl || ""}`.replace(/\/+$/, ""),
      authorizationId: "6045cda1d0081238ab8fd09b",
      tenantId: request.tenantId,
      tenantName: "HIHIS（一体化版）",
      userId: "6045cda1d0081238ab8fd09a",
      userName: "管理员",
      roleId: "6045cda1d0081238ab8fd099",
      roleName: "租户管理员",
      roleCd: "tenantSystem",
      message: "system 租户管理员认证通过，tk 登录凭证已建立（浏览器预览）",
    };
  }
  if (name === "load_target_dictionaries") return mockTargetDictionaryCatalog();
  if (name === "load_medicine_cost_merges") return mockMedicineCostMerges();
  if (name === "inspect_phis27_source")
    return {
      detected: true,
      adapterId: "PHIS27",
      title: "Bsoft PHIS27 药品数据",
      schema: args.profile?.schema || args.profile?.username || "PHIS27",
      checkedTables: [
        "YK_TYPK",
        "YK_YPBM",
        "YK_YPCD",
        "YK_CDDZ",
        "YK_YPXX",
        "YK_CDXX",
        "YK_KCMX",
        "YK_YKLB",
        "YF_YPXX",
        "YF_KCMX",
        "YF_YFLB",
        "YK_YPSX",
      ],
      missingTables: [],
      totalMedicines: 12646,
      configuredMedicines: 633,
      activeConfiguredMedicines: 617,
      productRows: 92,
      stockMedicines: 73,
      duplicateBusinessGroups: 3,
      orphanPharmacyConfigs: 25,
      scopes: [
        {
          id: "USED_ACTIVE",
          label: "机构在用药品",
          description: "同步药房已配置且未作废的药品；包含关联厂家商品，推荐首次迁移使用",
          estimatedRows: 622,
          medicineCount: 617,
          recommended: true,
        },
        {
          id: "USED_ALL",
          label: "机构全部配置药品",
          description: "包含已作废药品，用于需要保留完整机构历史字典的场景",
          estimatedRows: 638,
          medicineCount: 633,
          recommended: false,
        },
      ],
      warnings: [
        "机构配置范围发现3组目标判重键重复药品；迁移将保留 YPXH 并在预校验阶段阻止自动合并",
        "发现25条药房药品配置无法关联药房，基础同步可继续，库存初始化前必须处理",
        "费用归并主键和剂型编码属于新系统字典，需在校验阶段设置默认值或值映射",
      ],
      message: "已识别 PHIS27 药品主数据结构，可使用内置安全查询模板",
    };
  if (name === "load_phis27_medicine") return demoPhis27Preview();
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
  if (name === "preview_overwrite_batch")
    return mockPreviewOverwrite(args.request);
  if (name === "execute_migration_batch") return mockExecute(args.request);
  if (name === "undo_migration_batch") return mockUndo(args.request);
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

function mockTargetDictionaryCatalog() {
  const dictionary = (dicId, items) => ({
    dicId,
    lastModify: Date.now(),
    total: items.length,
    items: items.map(([key, text], index) => ({
      id: `preview-${dicId}-${index}`,
      key,
      cd: key,
      text,
      na: text,
      active: true,
    })),
  });
  return {
    ok: true,
    bindings: [
      { targetField: "sdMed", dictionaryId: "rbmh.base.med.articleType" },
      { targetField: "sdDose", dictionaryId: "rbmh.base.med.doseType" },
    ],
    dictionaries: [
      dictionary("rbmh.base.med.articleType", [
        ["1", "西药"],
        ["2", "中药"],
        ["3", "草药"],
        ["4", "疫苗"],
        ["5", "医用耗材"],
        ["6", "固定资产"],
        ["7", "民族药品"],
        ["8", "院内制剂"],
      ]),
      dictionary("rbmh.base.med.doseType", [
        ["1", "片剂"],
        ["2", "胶囊剂"],
        ["3", "注射剂"],
      ]),
    ],
    warnings: ["浏览器预览只展示物品类型和剂型示例，桌面版读取真实租户字典"],
    message: "浏览器预览：已加载 2 个药品标准字典",
  };
}

function mockMedicineCostMerges() {
  const items = [
    ["63aa8b1b3c6f491981ba4221", "西药费"],
    ["63aa8b1b3c6f491981ba4223", "成药费"],
    ["63aa8b1b3c6f491981ba422d", "草药费"],
    ["63aa8b1b3c6f491981ba4225", "卫生材料费"],
    ["63d9cd2f2e90de960c1bc2f1", "疫苗费"],
  ].map(([key, text], index) => ({
    key,
    text,
    leaf: true,
    parent: "",
    index,
    properties: { sdCstmg: "1" },
    active: true,
    mcode: "",
  }));
  return {
    items,
    total: items.length,
    message: "浏览器预览：已读取费用归并项目",
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

function demoPhis27Preview() {
  const rows = demoRows().map((row, index) => ({
    SOURCE_KEY: `${1001 + index}:${2001 + index}`,
    SOURCE_MED_ID: `${1001 + index}`,
    SOURCE_DUPLICATE_COUNT: "1",
    DRUG_NAME: row.DRUG_NAME,
    DRUG_TYPE: row.DRUG_TYPE,
    FORM_CODE: row.FORM_CODE,
    PRE_UNIT: row.PRE_UNIT,
    DOSE: row.DOSE,
    DOSE_UNIT: row.DOSE_UNIT,
    SPEC: row.SPEC,
    USAGE_CODE: row.USAGE_CODE,
    FREQ_CODE: row.FREQ_CODE,
    SOURCE_FACTORY_ID: `${2001 + index}`,
    FACTORY_NAME: row.FACTORY_NAME,
    PRODUCT_NAME: row.PRODUCT_NAME,
    SALE_UNIT: row.SALE_UNIT,
    PACK_FACTOR: row.PACK_FACTOR,
    SALE_SPEC: row.SPEC,
    BUY_PRICE: row.BUY_PRICE,
    RETAIL_PRICE: row.RETAIL_PRICE,
    APPROVAL_NO: row.APPROVAL_NO,
    BARCODE: "",
    CD_MED_PRO: `${1001 + index}-${2001 + index}`,
    RX_FLAG: "1",
    BASIC_DRUG_TYPE: "",
    SPECIAL_DRUG_TYPE: "",
    SOURCE_STOP_FLAG: "0",
  }));
  return {
    columns: Object.keys(rows[0]),
    rows,
    truncated: false,
    elapsedMs: 48,
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
    if (!`${normalized.idCstmg ?? ""}`.trim()) {
      normalized.idCstmg =
        request.costMergeMappings?.[`${normalized.sdMed ?? ""}`] || null;
    }
    const errors = [];
    ["naMed", "sdMed", "idCstmg", "sdDose", "unitPre"].forEach((key) => {
      if (
        normalized[key] === undefined ||
        normalized[key] === null ||
        `${normalized[key]}`.trim() === ""
      )
        errors.push(`${key}不能为空`);
    });
    const hasProduct = [
      "idFac",
      "naFac",
      "naMedPro",
      "unitSale",
      "unitSaleFactor",
      "specSale",
      "priceSale",
      "pricePur",
      "cdAppr",
      "cdBar",
      "cdMedPro",
    ].some((key) => `${normalized[key] ?? ""}`.trim() !== "");
    if (hasProduct) {
      ["unitSale", "unitSaleFactor", "pricePur", "priceSale"].forEach(
        (key) => {
          if (`${normalized[key] ?? ""}`.trim() === "")
            errors.push(`${key}不能为空`);
        },
      );
      if (!`${normalized.idFac ?? ""}`.trim() && !`${normalized.naFac ?? ""}`.trim())
        errors.push("生产厂家主键或名称至少填写一项");
    }
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
    conflictStrategy: request.conflictStrategy || "INCREMENTAL",
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
  const selected = new Set(request.selectedRowIds || []);
  detail.rows = detail.rows.map((row) => {
    if (selected.size && !selected.has(row.rowId)) return row;
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
  const pendingCount = detail.rows.filter(
    (row) => row.status === "VALIDATED",
  ).length;
  const successCount = detail.rows.filter(
    (row) => row.status === "SUCCESS",
  ).length;
  detail.batch = {
    ...detail.batch,
    status: pendingCount ? "PARTIAL" : "SUCCESS",
    validCount: pendingCount,
    successCount,
    failCount: detail.rows.filter((row) =>
      ["FAILED", "INVALID"].includes(row.status),
    ).length,
    updatedAt: now,
    finishedAt: now,
  };
  mockBatches.set(request.batchId, detail);
  return detail;
}

function mockPreviewOverwrite(request) {
  const detail = mockBatches.get(request.batchId);
  if (!detail) throw new Error("找不到要预览的覆盖批次");
  const rows = detail.rows
    .filter((row) => row.status === "VALIDATED")
    .map((row, index) => ({
      rowId: row.rowId,
      rowNo: row.rowNo,
      sourceKey: row.sourceKey,
      action: row.idMed ? "UPDATE" : "INSERT",
      changes: row.idMed
        ? [
            {
              table: "hi_bd_med",
              column: "na_med",
              label: "医疗物品通用名",
              before: `原药品${index + 1}`,
              after: row.normalizedData.naMed,
            },
          ]
        : [],
      message: row.idMed
        ? "浏览器预览：检测到 1 个字段变化"
        : "浏览器预览：新来源记录，将按普通新增迁移执行",
    }));
  const insertCount = rows.filter((row) => row.action === "INSERT").length;
  const updateCount = rows.filter((row) => row.action === "UPDATE").length;
  const unchangedCount = rows.filter(
    (row) => row.action === "UNCHANGED",
  ).length;
  const changedFieldCount = rows.reduce(
    (count, row) => count + row.changes.length,
    0,
  );
  return {
    batchId: request.batchId,
    rows,
    insertCount,
    updateCount,
    unchangedCount,
    changedFieldCount,
    message: `差异读取完成：新增${insertCount}条，覆盖${updateCount}条，无变化${unchangedCount}条，共${changedFieldCount}个字段变化`,
  };
}

function mockUndo(request) {
  const detail = mockBatches.get(request.batchId);
  if (!detail) throw new Error("找不到要撤销的迁移批次");
  const now = new Date().toISOString();
  detail.audits.unshift({
    auditId: objectId(),
    batchId: request.batchId,
    rowId: "",
    traceId: objectId(),
    operation: "UNDO",
    targetTable: "migration_batch",
    targetId: request.batchId,
    result: "UNDONE",
    beforeData: null,
    afterData: { deleted: detail.batch.successCount, retained: 0 },
    message: `浏览器预览：已模拟撤销 ${detail.batch.successCount} 行新增数据`,
    operatorId: "demo",
    operatedAt: now,
  });
  detail.batch = {
    ...detail.batch,
    status: "UNDONE",
    updatedAt: now,
    finishedAt: now,
  };
  mockBatches.set(request.batchId, detail);
  return detail;
}
