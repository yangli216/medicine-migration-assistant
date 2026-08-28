const databaseFamilyNames = {
  mysql: "MySQL",
  oracle: "Oracle",
  dameng: "达梦 DM8",
  opengauss: "Gauss / openGauss",
  vastbase: "海量 Vastbase",
  gbase8c: "GBase 8c",
  gbase8a: "GBase 8a",
  gbase8s: "GBase 8s",
  kingbase: "人大金仓 KingbaseES",
  postgresql: "PostgreSQL",
};

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}

function supportsTask(adapter, task) {
  return Array.isArray(adapter?.migrationTasks) && adapter.migrationTasks.includes(task);
}

function adapterRouteLabel(adapter) {
  const families = unique(adapter.databaseFamilies || []).map(
    (family) => databaseFamilyNames[family] || family,
  );
  return families.length
    ? `${adapter.name}（${families.join("、")}）`
    : adapter.name;
}

function adapterCompatibilityText(adapter) {
  const policy = adapter?.sourceCompatibility;
  if (!policy) {
    return `${adapter.name}：未声明厂商版本认证范围，接入前仍需完成来源结构验收。`;
  }
  const product = policy.product || adapter.name;
  if (policy.mode === "DECLARED_VERSION_RANGE") {
    const versions = (policy.declaredVersions || []).join("、") || "未列明";
    return `${product}：已声明验证范围 ${versions}；${policy.gate}`;
  }
  return `${product}：按现场结构契约判定；${policy.gate}`;
}

export function sourceAdapterCapabilityAssessment(
  adapters = [],
  task = "MEDICINE_BASE",
) {
  const automatic = adapters.filter(
    (adapter) => adapter.automaticDetection && supportsTask(adapter, task),
  );
  const automaticLabels = automatic.map(adapterRouteLabel);
  const automaticCompatibility = automatic.map(adapterCompatibilityText);

  if (task === "INVENTORY") {
    return {
      task,
      headline: automatic.length
        ? `当前 ${automatic.length} 个专用适配器可直接读取库存`
        : "当前没有可直接读取库存的适配器",
      summary:
        "库存涉及机构、库房、包装、金额和首次盘点，不使用通用 SQL 猜测业务关系。",
      routes: [
        {
          key: "automatic",
          tone: automatic.length ? "success" : "warning",
          label: "可直接接入",
          title: automaticLabels.join("、") || "暂无专用库存适配器",
          detail: automatic.length
            ? "连接数据库不代表 HIS 版本已认证；通过当前任务的来源结构门禁后，才能进入机构和库房映射。"
            : "需先完成对应 HIS 的库存适配器开发与验收。",
          compatibility: automaticCompatibility,
        },
        {
          key: "unsupported",
          tone: "warning",
          label: "其他 HIS",
          title: "不能仅因数据库可连接就直接迁移",
          detail:
            "即使是 Oracle、MySQL 或国产数据库，仍需确认 HIS 业务表关系并开发专用适配器。",
        },
        {
          key: "prerequisite",
          tone: "info",
          label: "必要前提",
          title: "先完成药品基础数据和来源台账",
          detail:
            "每条库存都必须能对应到新系统药品与厂家商品后才能进入首次盘点。",
        },
      ],
    };
  }

  const genericDatabase = adapters.find(
    (adapter) =>
      !adapter.automaticDetection &&
      adapter.sourceModes?.includes("database") &&
      supportsTask(adapter, "MEDICINE_BASE"),
  );
  const genericFile = adapters.find(
    (adapter) =>
      !adapter.automaticDetection &&
      adapter.sourceModes?.includes("file") &&
      supportsTask(adapter, "MEDICINE_BASE"),
  );
  const genericFamilies = unique(genericDatabase?.databaseFamilies || []).map(
    (family) => databaseFamilyNames[family] || family,
  );

  return {
    task: "MEDICINE_BASE",
    headline: "药品基础数据有三种可选接入路径",
    summary:
      "有专用适配器时可自动识别；没有时仍可通过只读数据库或文件完成字段映射。",
    routes: [
      {
        key: "automatic",
        tone: automatic.length ? "success" : "info",
        label: "自动识别",
        title: automaticLabels.join("、") || "暂无专用药品适配器",
        detail: automatic.length
          ? "连接数据库不代表 HIS 版本已认证；适配器会按声明的兼容策略核对后再提供预设映射。"
          : "可改用下方通用映射路径。",
        compatibility: automaticCompatibility,
      },
      {
        key: "database",
        tone: genericDatabase ? "info" : "warning",
        label: "通用数据库",
        title: genericDatabase
          ? `${genericFamilies.length} 类数据库可用表/视图或只读 SQL 接入`
          : "当前未注册通用数据库路径",
        detail:
          "需实施人员确认稳定业务键、字段含义和字典对应，确认后可保存为本地模板复用。",
      },
      {
        key: "file",
        tone: genericFile ? "info" : "warning",
        label: "文件交接",
        title: genericFile ? "CSV / JSON 可直接进入引导映射" : "当前未注册文件路径",
        detail:
          "适合由 HIS 厂商导出稳定药品清单的项目，不需要开放数据库连接。",
      },
    ],
  };
}
