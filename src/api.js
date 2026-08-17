import { invoke } from "@tauri-apps/api/core";
import { applyFieldMapping } from "./transforms";

export const isDesktop =
  typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);

const mockBatches = new Map();
let mockSavedConnections = {
  source: null,
  targetDatabase: null,
  targetSystem: null,
};
let mockDatabaseConnections = [];
let mockPhis27MappingProfile = null;
let mockInventoryLocationMappings = [];
let mockInventoryOrganizationMappings = [];

export async function command(name, args = {}) {
  if (isDesktop) return invoke(name, args);
  if (name === "load_saved_connections") return mockSavedConnections;
  if (name === "list_database_connections") return mockDatabaseConnections;
  if (name === "save_database_connection") {
    const connectionId = args.request.connectionId || objectId();
    const saved = {
      connectionId,
      name: args.request.name,
      purpose: args.request.purpose,
      profile: { ...args.request.profile },
      rememberPassword: args.request.rememberPassword,
      passwordAvailable: Boolean(
        args.request.rememberPassword && args.request.profile.password,
      ),
      updatedAt: new Date().toISOString(),
    };
    const index = mockDatabaseConnections.findIndex(
      (entry) => entry.connectionId === connectionId,
    );
    mockDatabaseConnections =
      index >= 0
        ? mockDatabaseConnections.map((entry, entryIndex) =>
            entryIndex === index ? saved : entry,
          )
        : [saved, ...mockDatabaseConnections];
    return saved;
  }
  if (name === "delete_database_connection") {
    mockDatabaseConnections = mockDatabaseConnections.filter(
      (entry) => entry.connectionId !== args.connectionId,
    );
    return null;
  }
  if (name === "load_phis27_mapping_profile")
    return mockPhis27MappingProfile;
  if (name === "save_phis27_mapping_profile") {
    mockPhis27MappingProfile = {
      version: 2,
      mapping: { ...args.request.mapping },
      rules: { ...args.request.rules },
      dictionaryOverrides: structuredClone(
        args.request.dictionaryOverrides || {},
      ),
      savedAt: new Date().toISOString(),
    };
    return mockPhis27MappingProfile;
  }
  if (name === "save_source_connection") {
    mockSavedConnections = {
      ...mockSavedConnections,
      source: {
        profile: args.request.profile,
        rememberPassword: args.request.rememberPassword,
        passwordAvailable: Boolean(
          args.request.rememberPassword && args.request.profile.password,
        ),
      },
    };
    return mockSavedConnections.source;
  }
  if (name === "save_target_database_connection") {
    mockSavedConnections = {
      ...mockSavedConnections,
      targetDatabase: {
        profile: args.request.profile,
        rememberPassword: args.request.rememberPassword,
        passwordAvailable: Boolean(
          args.request.rememberPassword && args.request.profile.password,
        ),
      },
    };
    return mockSavedConnections.targetDatabase;
  }
  if (name === "save_target_system_connection") {
    mockSavedConnections = {
      ...mockSavedConnections,
      targetSystem: {
        ...args.request,
        passwordAvailable: Boolean(
          args.request.rememberPassword && args.request.password,
        ),
      },
    };
    return mockSavedConnections.targetSystem;
  }
  if (name === "forget_source_connection") {
    mockSavedConnections = { ...mockSavedConnections, source: null };
    return null;
  }
  if (name === "forget_target_database_connection") {
    mockSavedConnections = { ...mockSavedConnections, targetDatabase: null };
    return null;
  }
  if (name === "forget_target_system_connection") {
    mockSavedConnections = { ...mockSavedConnections, targetSystem: null };
    return null;
  }
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
        "vastbase",
        "gbase8c",
        "gbase8a",
        "gbase8s",
      ],
    };
  if (name === "list_database_drivers") return ["Oracle 19 ODBC driver"];
  if (name === "list_driver_packs")
    return [
      mockDriverPack("mysql", "MySQL 原生驱动", "内置", "bundled", ""),
      mockDriverPack(
        "oracle",
        "Oracle Instant Client ODBC 19.31",
        "19.31 · Windows x64",
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
        "openGauss / GaussDB PostgreSQL 协议",
        "应用内置",
        "bundled",
        "",
      ),
      mockDriverPack(
        "kingbase",
        "KingbaseES PostgreSQL 协议",
        "应用内置优先",
        "bundled",
        "",
      ),
      mockDriverPack(
        "postgresql",
        "PostgreSQL 通用协议",
        "应用内置",
        "bundled",
        "",
      ),
      mockDriverPack("vastbase", "海量 Vastbase G100 PostgreSQL 协议", "应用内置", "bundled", ""),
      mockDriverPack("gbase8c", "南大通用 GBase 8c PostgreSQL 协议", "应用内置", "bundled", ""),
      mockDriverPack("gbase8a", "南大通用 GBase 8a ODBC", "随服务端版本", "profile-ready", "GBase ODBC 8.3 Driver"),
      mockDriverPack("gbase8s", "南大通用 GBase 8s ODBC", "随服务端版本", "profile-ready", "GBase ODBC DRIVER"),
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
      title: "二系列phis药品数据",
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
      configuredMedicines: 85,
      activeConfiguredMedicines: 85,
      productRows: 92,
      stockMedicines: 73,
      duplicateBusinessGroups: 0,
      orphanPharmacyConfigs: 0,
      scopes: [
        {
          id: "USED_ACTIVE",
          label: "机构在用药品",
          description:
            "仅以 YK_CDXX 中未作废的机构产地配置判断在用；不关联药库或药房药品配置表",
          estimatedRows: 90,
          medicineCount: 85,
          recommended: true,
        },
        {
          id: "USED_ALL",
          label: "机构全部配置药品",
          description:
            "包含 YK_CDXX 中曾配置的全部药品及产地，不关联 YK_YPXX、YF_YPXX",
          estimatedRows: 90,
          medicineCount: 85,
          recommended: false,
        },
        {
          id: "ALL_MEDICINES",
          label: "全部通用药品",
          description: "读取药品主表中的全部通用药品，不受机构配置和作废状态限制",
          estimatedRows: 12646,
          medicineCount: 12646,
          recommended: false,
        },
      ],
      warnings: [
        "费用归并主键和剂型编码属于新系统字典，需在校验阶段设置默认值或值映射",
      ],
      message: "已识别二系列phis药品主数据结构，可使用内置安全查询模板",
    };
  if (name === "inspect_phis27_inventory")
    return {
      readyForLocationMapping: true,
      schema: args.request?.connection?.schema || "PHIS27",
      sourceName: args.request?.sourceName || "二系列phis演示库",
      stockRowCount: 118,
      stockGroupCount: 107,
      medicineCount: 81,
      mappedMedicineCount: 81,
      unresolvedMedicineCount: 0,
      locations: [
        {
          sourceKind: "WAREHOUSE",
          sourceLocationKey: "YK:1001",
          sourceLocationName: "中心药库",
          organizationId: "420100001",
          sourceOptionCount: 1,
          stockRowCount: 74,
          stockGroupCount: 68,
          medicineCount: 58,
          mappingStatus: "PENDING_TARGET_MAPPING",
          mappingMessage: "老系统位置已识别，下一步选择对应的新系统库房",
        },
        {
          sourceKind: "PHARMACY",
          sourceLocationKey: "YF:1001",
          sourceLocationName: "门诊西药房",
          organizationId: "420100001",
          sourceOptionCount: 1,
          stockRowCount: 32,
          stockGroupCount: 28,
          medicineCount: 24,
          mappingStatus: "PENDING_TARGET_MAPPING",
          mappingMessage: "老系统位置已识别，下一步选择对应的新系统库房",
        },
        {
          sourceKind: "PHARMACY",
          sourceLocationKey: "YF:2001",
          sourceLocationName: "第二机构西药房",
          organizationId: "420100002",
          sourceOptionCount: 1,
          stockRowCount: 12,
          stockGroupCount: 11,
          medicineCount: 8,
          mappingStatus: "PENDING_TARGET_MAPPING",
          mappingMessage: "老系统位置已识别，可在后续批次继续映射",
        },
      ],
      unresolvedSourceKeys: [],
      warnings: [],
      message:
        "库存药品已全部关联到本次核实的基础数据台账，可以进入库房映射配置",
    };
  if (name === "load_phis27_inventory_catalog")
    return {
      organizations: [
        {
          id: "420100001",
          name: "老系统示范医院",
          parentId: "420100",
          organizationType: "1",
          active: true,
        },
        {
          id: "420100002",
          name: "老系统第二机构",
          parentId: "420100",
          organizationType: "1",
          active: true,
        },
      ],
      locations: [
        {
          sourceKind: "WAREHOUSE",
          sourceLocationKey: "YK:1001",
          id: "1001",
          name: "中心药库",
          organizationId: "420100001",
          category: "1",
          active: true,
        },
        {
          sourceKind: "PHARMACY",
          sourceLocationKey: "YF:1001",
          id: "1001",
          name: "门诊西药房",
          organizationId: "420100001",
          category: "",
          active: true,
        },
        {
          sourceKind: "PHARMACY",
          sourceLocationKey: "YF:2001",
          id: "2001",
          name: "第二机构西药房",
          organizationId: "420100002",
          category: "",
          active: true,
        },
      ],
      warnings: [],
      message: "浏览器预览：已读取 2 个老系统机构、3 个药库/药房",
    };
  if (name === "load_inventory_target_organizations")
    return {
      organizations: [
        {
          id: "5b7e12988ddd9446d42a03c8",
          name: "浙一医院",
          cd: "330101",
          orgId: "5b7e12988ddd9446d42a03c8",
          tenantId: "zhongshan",
          orgType: "00",
          orgTypeText: "系统主机构",
          fullName: "浙一医院",
          parent: "5b7d135c8ddd9418e843424e",
          parentText: "杭州市卫生局",
          active: true,
        },
        {
          id: "63f2de263c6f4958b32eebfc",
          name: "浦沿街道社区卫生服务中心2",
          cd: "330108001",
          orgId: "63f2de263c6f4958b32eebfc",
          tenantId: "zhongshan",
          orgType: "00",
          orgTypeText: "系统主机构",
          fullName: "浦沿街道社区卫生服务中心2",
          parent: "5b7d135c8ddd9418e843424e",
          parentText: "滨江区卫生健康局",
          active: true,
        },
      ],
      message: "浏览器预览：已从新系统服务读取 2 个有效机构",
    };
  if (name === "load_inventory_target_storages")
    return {
      storages: [
        {
          idSto: "64b2fca10a1f2e3d4c5b6001",
          name: "中心药库",
          storageType: "1",
          storageTypeName: "药库",
          productTypes: "药品",
          organizationId: "5b7e12988ddd9446d42a03c8",
        },
        {
          idSto: "64b2fca10a1f2e3d4c5b6002",
          name: "门诊西药房",
          storageType: "2",
          storageTypeName: "药房",
          productTypes: "药品",
          organizationId: "5b7e12988ddd9446d42a03c8",
        },
      ],
      message: "浏览器预览：已读取 2 个有效新系统仓储",
    };
  if (name === "load_inventory_location_mappings")
    return mockInventoryLocationMappings;
  if (name === "load_inventory_organization_mappings")
    return mockInventoryOrganizationMappings;
  if (name === "prepare_phis27_inventory") {
    mockInventoryLocationMappings = args.request.mappings.map((mapping) => ({
      ...mapping,
    }));
    mockInventoryOrganizationMappings = args.request.organizationMappings.map(
      (mapping) => ({ ...mapping }),
    );
    return mockPrepareInventory(args.request);
  }
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
  if (name === "execute_phis27_inventory")
    return mockExecuteInventory(args.request);
  if (name === "undo_migration_batch") return mockUndo(args.request);
  if (name === "load_migration_batch") return mockBatches.get(args.batchId);
  if (name === "list_recent_batches")
    return [...mockBatches.values()]
      .map((item) => item.batch)
      .sort((left, right) => right.createdAt.localeCompare(left.createdAt))
      .slice(0, args.limit || 30);
  throw new Error(`浏览器预览未实现命令：${name}`);
}

function mockPrepareInventory(request) {
  const batchId = objectId();
  const now = new Date().toISOString();
  const rows = request.mappings.map((mapping, index) => ({
    rowId: objectId(),
    batchId,
    rowNo: index + 1,
    sourceKey: `${mapping.sourceKind === "WAREHOUSE" ? "YK" : "YF"}:${1001 + index}`,
    sourceHash: objectId(),
    status: "VALIDATED",
    rawData: {
      sourceLocationKey: mapping.sourceLocationKey,
      sourceLocationName: mapping.sourceLocationName,
      sourceProductKey: `${26911 + index}:${12000 + index}`,
      drugName: index % 2 ? "阿莫西林胶囊" : "维生素D滴剂",
      specification: index % 2 ? "0.25g*24粒/盒" : "400IU*36粒/盒",
      minimumUnit: "粒",
      saleSpecification: index % 2 ? "0.25g*24粒/盒" : "400IU*36粒/盒",
      saleUnit: index % 2 ? "盒" : "粒",
      unitSaleFactor: index % 2 ? "24" : "1",
      productSaleUnit: "盒",
      productUnitSaleFactor: index % 2 ? "24" : "36",
      productName: index % 2 ? "阿莫西林胶囊" : "维生素D滴剂",
      factoryName: "示例生产厂家",
      packagingNotes: index % 2
        ? []
        : ["已采用当前药房的实际包装：粒×1；商品主档为 盒×36"],
    },
    normalizedData: {
      idSto: mapping.targetIdSto,
      naSto: mapping.targetName,
      idOrg: mapping.targetIdOrg,
      amount: `${28 + index}`,
      pricePur: "1.20",
      priceSale: "1.50",
      unitSale: index % 2 ? "盒" : "粒",
      unitSaleFactor: index % 2 ? "24" : "1",
      specSale: index % 2 ? "0.25g*24粒/盒" : "400IU*36粒/盒",
      purchaseTotal: `${(28 + index) * 1.2}`,
      retailTotal: `${(28 + index) * 1.5}`,
      batchCode: `B20260${index + 1}`,
      effectiveDate: "2028-12-31",
    },
    errorCode: "",
    errorMessage: "",
    idMed: objectId(),
    idMedUnit: objectId(),
    idFac: objectId(),
    idMedPro: objectId(),
    retryCount: 0,
    updatedAt: now,
  }));
  const detail = {
    batch: {
      batchId,
      batchName: "二系列phis库存预检-浏览器预览",
      sourceType: "PHIS27_INVENTORY",
      sourceName: request.sourceName,
      sourceDescription: "YK_KCMX/YF_KCMX 非零库存防重预检",
      conflictStrategy: "FAIL",
      allowCreateFactory: false,
      idempotencyKey: objectId(),
      status: "VALIDATED",
      totalCount: rows.length,
      validCount: rows.length,
      successCount: 0,
      failCount: 0,
      skipCount: 0,
      createdAt: now,
      updatedAt: now,
      finishedAt: null,
    },
    rows,
    audits: [],
  };
  mockBatches.set(batchId, detail);
  return detail;
}

function mockExecuteInventory(request) {
  const detail = mockBatches.get(request.batchId);
  if (!detail || detail.batch.sourceType !== "PHIS27_INVENTORY") {
    throw new Error("浏览器预览中未找到对应的库存预检批次");
  }
  const now = new Date();
  const prefix = `${now.getFullYear()}${`${now.getMonth() + 1}`.padStart(2, "0")}${`${now.getDate()}`.padStart(2, "0")}`;
  const numberByStorage = new Map();
  const rows = detail.rows.map((row) => {
    if (row.status !== "VALIDATED") return row;
    const idSto = row.normalizedData?.idSto;
    if (!numberByStorage.has(idSto)) {
      numberByStorage.set(idSto, `${prefix}001`);
    }
    return {
      ...row,
      status: "SUCCESS",
      errorCode: "",
      errorMessage: `首次盘点 ${numberByStorage.get(idSto)} 建立初始账簿`,
      updatedAt: now.toISOString(),
    };
  });
  const audits = [...numberByStorage.entries()].map(([idSto, cdStoCheck]) => ({
    auditId: objectId(),
    batchId: request.batchId,
    rowId: "",
    traceId: objectId(),
    operation: "INSERT",
    targetTable: "hi_sto_check",
    targetId: objectId(),
    result: "SUCCESS",
    beforeData: null,
    afterData: { idSto, cdStoCheck, fgStoCheck: "1", sdCheck: "1" },
    message: `首次盘点单 ${cdStoCheck} 已完成`,
    operatorId: "preview-system",
    operatedAt: now.toISOString(),
  }));
  const successCount = rows.filter((row) => row.status === "SUCCESS").length;
  const next = {
    ...detail,
    batch: {
      ...detail.batch,
      status: "SUCCESS",
      validCount: 0,
      successCount,
      failCount: 0,
      updatedAt: now.toISOString(),
      finishedAt: now.toISOString(),
    },
    rows,
    audits,
  };
  mockBatches.set(request.batchId, next);
  return next;
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
    licenseNote: state === "bundled" ? "通用协议随应用内置，无需单独安装。" : "连接配置已预置，桌面版会检测本机实际安装状态。",
    officialUrl: "https://www.postgresql.org/docs/current/protocol.html",
    protocol: state === "bundled" ? "postgresql-wire" : "odbc",
    installGuide: state === "bundled" ? "无需安装，填写连接参数即可测试。" : "安装与数据库服务端版本及操作系统架构一致的 64 位厂商 ODBC 驱动。",
    platformPriority: ["windows-x64", "macos-arm64", "macos-x64"],
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
      { targetField: "fgMedRx", dictionaryId: "rbmh.base.med.prescriptiondrugIdentification" },
      { targetField: "sdChrgitmLv", dictionaryId: "phis.medicareLevel" },
      { targetField: "sdAllergy", dictionaryId: "rbmh.base.med.sdAllergy" },
      { targetField: "sdStorage", dictionaryId: "phis.storageType" },
      { targetField: "sdRound", dictionaryId: "rbmh.base.med.roundingStrategy" },
      { targetField: "sdDps", dictionaryId: "rbmh.base.med.dispensingMethod" },
      { targetField: "dftUsage", dictionaryId: "rbmh.base.med.usage" },
      { targetField: "dftFreq", dictionaryId: "rbmh.base.freq" },
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
      dictionary("rbmh.base.med.prescriptiondrugIdentification", [
        ["1", "处方药"],
        ["2", "非处方药"],
      ]),
      dictionary("phis.medicareLevel", [
        ["01", "甲类"],
        ["02", "乙类"],
        ["03", "丙类"],
      ]),
      dictionary("rbmh.base.med.sdAllergy", [
        ["1", "青霉素"],
        ["2", "磺胺"],
        ["3", "喹诺酮"],
        ["4", "头孢"],
        ["5", "四环素"],
        ["6", "抗生素"],
        ["7", "血清制剂"],
        ["8", "链霉素"],
      ]),
      dictionary("phis.storageType", [
        ["1", "常温"],
        ["2", "阴凉"],
        ["3", "低温"],
      ]),
      dictionary("rbmh.base.med.roundingStrategy", [
        ["1", "每次发药数量取整"],
        ["2", "每天发药数量取整"],
        ["3", "不取整"],
      ]),
      dictionary("rbmh.base.med.dispensingMethod", [
        ["1", "药房发药"],
        ["2", "病区发药"],
        ["3", "门诊发药"],
        ["9", "其他"],
      ]),
      dictionary("rbmh.base.med.usage", [
        ["100", "口服"],
        ["402", "静脉滴注"],
      ]),
      dictionary("rbmh.base.freq", [
        ["QD", "每日一次"],
        ["TID", "每日三次"],
      ]),
    ],
    warnings: ["浏览器预览使用演示字典，桌面版读取真实租户字典"],
    message: "浏览器预览：已加载药品标准字典",
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
    SOURCE_DUPLICATE_COUNT: index < 2 ? "2" : "1",
    DRUG_NAME: row.DRUG_NAME,
    DRUG_TYPE: row.DRUG_TYPE,
    FORM_CODE: row.FORM_CODE,
    PRE_UNIT: row.PRE_UNIT,
    T__YPDW: row.SALE_UNIT,
    DOSE: row.DOSE,
    DOSE_UNIT: row.DOSE_UNIT,
    SPEC: row.SPEC,
    USAGE_CODE: `${(index % 2) + 1}`,
    FREQ_CODE: row.FREQ_CODE,
    DOSE_ONCE: row.DOSE,
    ROUND_CODE: `${index % 3}`,
    DISPENSE_CODE: `${(index % 2) + 1}`,
    SOURCE_FACTORY_ID: `${2001 + index}`,
    SOURCE_PRODUCT_ID: `${3001 + index}`,
    SOURCE_MED_PRO_KEY: `${1001 + index}:${2001 + index}`,
    FACTORY_NAME: row.FACTORY_NAME,
    FACTORY_SHORT_NAME: row.FACTORY_NAME.slice(0, 16),
    FACTORY_PINYIN: ["SYJTOYYY", "JSHRYY", "ASLKZY"][index],
    PRODUCT_NAME: row.PRODUCT_NAME,
    SALE_UNIT: row.SALE_UNIT,
    PACK_FACTOR: row.PACK_FACTOR,
    SALE_SPEC: row.SPEC,
    BUY_PRICE: row.BUY_PRICE,
    RETAIL_PRICE: row.RETAIL_PRICE,
    APPROVAL_NO: row.APPROVAL_NO,
    BARCODE: "",
    CD_MED_PRO: `${1001 + index}:${2001 + index}`,
    RX_FLAG: "1",
    BASIC_DRUG_TYPE: `${(index % 4) + 1}`,
    INSURANCE_LEVEL: `${(index % 3) + 1}`,
    ORIGIN_TYPE: `${(index % 3) + 1}`,
    STORAGE_CODE: `${(index % 3) + 1}`,
    SPECIAL_DRUG_TYPE: ["6", "3", "8"][index],
    ALLERGY_CODE: ["1", "4", "0"][index],
    ANTI_APPROVAL: `${(index % 2) + 1}`,
    ANTIBIOTIC_FLAG: `${index % 2}`,
    DAILY_LIMIT: ["3", "2", "1"][index],
    SOURCE_STOP_FLAG: "0",
  }));
  const metadata = {
    DRUG_NAME: ["药品名称", "YK_TYPK", "YPMC"],
    DRUG_TYPE: ["药品类型", "YK_TYPK", "TYPE"],
    FORM_CODE: ["剂型编码", "YK_TYPK", "YPSX"],
    PRE_UNIT: ["最小单位（制剂单位）", "YK_TYPK", "ZXDW"],
    T__YPDW: ["药品单位", "YK_TYPK", "YPDW"],
    DOSE: ["制剂剂量", "YK_TYPK", "YPJL"],
    DOSE_UNIT: ["剂量单位", "YK_TYPK", "JLDW"],
    SPEC: ["制剂规格", "YK_TYPK", "YPGG"],
    USAGE_CODE: ["默认给药方法", "YK_TYPK", "GYFF"],
    FREQ_CODE: ["默认频次", "YK_TYPK", "MRYF"],
    DOSE_ONCE: ["默认一次剂量", "YK_TYPK", "YCJL"],
    ROUND_CODE: ["取整策略", "YK_TYPK", "QZCL"],
    DISPENSE_CODE: ["发药方式", "YK_TYPK", "FYFS"],
    FACTORY_NAME: ["生产厂家全称（为空时使用 CDMC）", "YK_CDDZ", "CDQC"],
    FACTORY_SHORT_NAME: ["生产厂家简称", "YK_CDDZ", "CDMC"],
    FACTORY_PINYIN: ["生产厂家拼音代码", "YK_CDDZ", "PYDM"],
    PRODUCT_NAME: ["商品名", "YK_YPCD", "YBSPMC"],
    SALE_UNIT: ["零售包装单位", "YK_TYPK", "YFDW"],
    PACK_FACTOR: ["包装系数", "YK_TYPK", "YFBZ"],
    SALE_SPEC: ["零售包装规格", "YK_TYPK", "YFGG"],
    BUY_PRICE: ["进货价格", "YK_YPCD", "JHJG"],
    RETAIL_PRICE: ["零售价格", "YK_YPCD", "LSJG"],
    APPROVAL_NO: ["批准文号", "YK_YPCD", "PZWH"],
    BARCODE: ["商品条形码", "YK_YPCD", "YPTM"],
    CD_MED_PRO: ["商品来源组合码", "", "YPXH:YPCD"],
    RX_FLAG: ["处方药标志", "YK_TYPK", "CFYP"],
    BASIC_DRUG_TYPE: ["基药类型", "YK_TYPK", "JYLX"],
    INSURANCE_LEVEL: ["医保分类", "YK_TYPK", "YBFL"],
    ORIGIN_TYPE: ["产地档次", "YK_TYPK", "YPDC"],
    STORAGE_CODE: ["药品贮藏", "YK_TYPK", "YPZC"],
    SPECIAL_DRUG_TYPE: ["特殊药品类型", "YK_TYPK", "TSYP"],
    ALLERGY_CODE: ["过敏药物类别", "YK_TYPK", "GMYWLB"],
    ANTI_APPROVAL: ["抗菌药物是否审批", "YK_TYPK", "SFSP"],
    ANTIBIOTIC_FLAG: ["是否抗生素", "YK_TYPK", "KSBZ"],
    DAILY_LIMIT: ["一日限量", "YK_TYPK", "YCYL"],
  };
  const sourceDictionaries = {
    DRUG_TYPE: ["处方类型", [["1", "西药"], ["2", "中药"], ["3", "草药"], ["9", "疫苗"]]],
    FORM_CODE: ["剂型", [["CAP", "胶囊剂"], ["INJ", "注射剂"]]],
    USAGE_CODE: ["使用途径", [["1", "口服"], ["2", "静滴"]]],
    FREQ_CODE: ["用药频次", [["QD", "每日一次"], ["TID", "每日三次"]]],
    ROUND_CODE: ["取整策略", [["0", "每次发药数量取整"], ["1", "每天发药数量取整"], ["2", "不取整"]]],
    DISPENSE_CODE: ["发药方式", [["1", "药房发药"], ["2", "病区发药"]]],
    RX_FLAG: ["处方药", [["1", "处方药品（RX）"], ["2", "非处方药品（OTC）"]]],
    BASIC_DRUG_TYPE: ["基药类型", [["1", "非基本药物"], ["2", "国家基本药物"], ["3", "省基本药物"], ["4", "区自选"]]],
    INSURANCE_LEVEL: ["医保分类", [["1", "甲类"], ["2", "乙类"], ["3", "丙类"]]],
    ORIGIN_TYPE: ["产地档次", [["1", "国产"], ["2", "合资"], ["3", "进口"]]],
    STORAGE_CODE: ["药品贮藏", [["1", "常温"], ["2", "阴凉"], ["3", "低温"]]],
    SPECIAL_DRUG_TYPE: ["特殊药品", [["1", "麻醉"], ["3", "贵重"], ["4", "毒性"], ["5", "放射"], ["6", "一般"], ["7", "一类精神"], ["8", "二类精神"]]],
    ALLERGY_CODE: ["过敏药物类别", [["1", "青霉素"], ["2", "磺胺"], ["3", "喹诺酮"], ["4", "头孢"]]],
    ANTI_APPROVAL: ["是否审批", [["1", "需要"], ["2", "不需要"]]],
    ANTIBIOTIC_FLAG: ["确认", [["0", "否"], ["1", "是"]]],
  };
  const dynamicDictionaryConfig = {
    FORM_CODE: ["YK_YPSX", "YPSX", "SXMC", []],
    USAGE_CODE: ["ZY_YPYF", "YPYF", "XMMC", ["PYDM", "FYXH", "BZYF"]],
    FREQ_CODE: ["GY_SYPC", "PCBM", "PCMC", ["MRCS", "ZXSJ", "ZXZQ", "RZXZQ"]],
    DISPENSE_CODE: ["ZY_FYFS", "FYFS", "FSMC", []],
  };
  const mockDictionaryProperties = {
    FREQ_CODE: {
      QD: { MRCS: "1", ZXSJ: "08:00", ZXZQ: "1", RZXZQ: "1" },
      TID: { MRCS: "3", ZXSJ: "08:00,12:00,18:00", ZXZQ: "1", RZXZQ: "1" },
    },
    USAGE_CODE: {
      1: { PYDM: "KF", BZYF: "1" },
      2: { PYDM: "JD", BZYF: "405" },
    },
  };
  const ineligible = new Set([
    "SOURCE_KEY",
    "SOURCE_MED_ID",
    "SOURCE_DUPLICATE_COUNT",
    "SOURCE_FACTORY_ID",
    "SOURCE_PRODUCT_ID",
    "SOURCE_MED_PRO_KEY",
    "SOURCE_STOP_FLAG",
  ]);
  return {
    columns: Object.keys(rows[0]),
    columnMetadata: Object.keys(rows[0]).map((name) => ({
      name,
      comment: metadata[name]?.[0] || "内部迁移辅助字段",
      sourceTable: metadata[name]?.[1] || "",
      sourceColumn: metadata[name]?.[2] || "",
      mappingEligible: !ineligible.has(name),
      sourceDictionary: sourceDictionaries[name]
        ? {
            id: `phis.mock.${name}`,
            name: sourceDictionaries[name][0],
            source: "二系列字典配置",
            entry: dynamicDictionaryConfig[name]?.[0] || "",
            keyField: dynamicDictionaryConfig[name]?.[1] || "",
            textField: dynamicDictionaryConfig[name]?.[2] || "",
            propertyFields: dynamicDictionaryConfig[name]?.[3] || [],
            loadStatus: dynamicDictionaryConfig[name] ? "loaded" : "bundled",
            loadMessage: dynamicDictionaryConfig[name]
              ? `已从 ${dynamicDictionaryConfig[name][0]} 读取字典项`
              : "来自二系列字典配置文件",
            items: sourceDictionaries[name][1].map(([key, text]) => ({
              key,
              text,
              properties: mockDictionaryProperties[name]?.[key] || {},
            })),
          }
        : null,
    })),
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
    const ignoredFields = [];
    request.mappings.forEach((mapping) => {
      const rawValue = raw[mapping.sourceField];
      const lookup = `${rawValue ?? ""}`.trim() || "<空值>";
      if (
        Object.prototype.hasOwnProperty.call(
          mapping.valueMappings || {},
          lookup,
        ) &&
        mapping.valueMappings[lookup] === null
      ) {
        ignoredFields.push(mapping.targetField);
      }
      normalized[mapping.targetField] = applyFieldMapping(raw, mapping);
    });
    if (ignoredFields.length) {
      normalized._ignoredValidationFields = ignoredFields;
    }
    if (`${raw.SOURCE_FACTORY_ID ?? ""}`.trim()) {
      normalized._sourceFactoryKey = `${raw.SOURCE_FACTORY_ID}`.trim();
    }
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
  const mergedMedicineIds = new Map();
  const invalidCount = detail.rows.filter(
    (row) => row.status === "INVALID",
  ).length;
  if (invalidCount && !request.skipInvalidRows) {
    throw new Error(
      `本批次还有 ${invalidCount} 条校验失败数据；请返回校验页修正，或明确选择“仅迁移校验通过的数据”`,
    );
  }
  if (request.skipInvalidRows) {
    detail.rows = detail.rows.map((row) => {
      if (row.status !== "INVALID") return row;
      detail.audits.unshift({
        auditId: objectId(),
        batchId: request.batchId,
        rowId: row.rowId,
        traceId: objectId(),
        operation: "VALIDATION_SKIP",
        targetTable: "migration_row",
        targetId: row.rowId,
        result: "SKIPPED",
        beforeData: {
          status: row.status,
          errorCode: row.errorCode,
          errorMessage: row.errorMessage,
        },
        afterData: {
          status: "SKIPPED",
          reason: "USER_CONFIRMED_VALIDATION_SKIP",
        },
        message: "用户确认仅迁移校验通过的数据，本行保留原校验原因并跳过写入",
        operatorId: request.operatorId,
        operatedAt: now,
      });
      return { ...row, status: "SKIPPED", updatedAt: now };
    });
  }
  detail.rows = detail.rows.map((row) => {
    if (selected.size && !selected.has(row.rowId)) return row;
    if (
      row.status !== "VALIDATED" &&
      !(request.failedOnly && row.status === "FAILED")
    )
      return row;
    const mergeKey = [
      row.normalizedData.naMed,
      row.normalizedData.spec,
      row.normalizedData.unitPre,
    ]
      .map((value) => `${value ?? ""}`.trim())
      .join("|");
    const reusedMedicine = mergedMedicineIds.has(mergeKey);
    const idMed = mergedMedicineIds.get(mergeKey) || objectId();
    mergedMedicineIds.set(mergeKey, idMed);
    const updated = {
      ...row,
      status: "SUCCESS",
      idMed,
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
      message: reusedMedicine
        ? "浏览器预览：按名称、规格、单位自动合并并保留来源映射"
        : "浏览器预览：模拟直接写表成功",
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
    skipCount: detail.rows.filter((row) => row.status === "SKIPPED").length,
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
