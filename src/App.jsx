import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CheckCircle,
  Clock,
  Code,
  Database,
  FileCsv,
  FirstAidKit,
  FloppyDisk,
  Gear,
  Info,
  LinkSimple,
  ListMagnifyingGlass,
  LockKey,
  Play,
  Plugs,
  Rows,
  ShieldCheck,
  Table,
  UploadSimple,
  UserCircle,
  Warning,
  X,
} from "@phosphor-icons/react";
import { command, demoRows, isDesktop, objectId } from "./api";
import { applyFieldRule, parseValueMappings } from "./transforms";

const steps = [
  "导入数据源",
  "识别数据",
  "匹配字段",
  "校验与修正",
  "执行与审计",
];

const targetFields = [
  {
    key: "naMed",
    label: "医疗物品通用名",
    required: true,
    group: "基本信息",
    hint: "写入 hi_bd_med.na_med",
    aliases: ["DRUG_NAME", "GENERIC_NAME", "YPMC", "药品名称"],
  },
  {
    key: "sdMed",
    label: "物品类型编码",
    required: true,
    group: "基本信息",
    hint: "1西药、2中成药、3草药、4保健品、5耗材、9其他",
    aliases: ["DRUG_TYPE", "SD_MED", "YPLX", "物品类型"],
  },
  {
    key: "idCstmg",
    label: "费用归并主键",
    required: true,
    group: "基本信息",
    hint: "新系统费用归并记录的24位主键",
    aliases: ["CSTMG_ID", "ID_CSTMG", "FYGB", "费用归并"],
  },
  {
    key: "sdDose",
    label: "剂型编码",
    required: true,
    group: "基本信息",
    hint: "需映射为新系统剂型字典编码",
    aliases: ["FORM_CODE", "DOSE_FORM", "JXDM", "剂型"],
  },
  {
    key: "unitPre",
    label: "制剂单位",
    required: true,
    group: "基本信息",
    hint: "最小制剂单位，例如片、粒、ml",
    aliases: ["PRE_UNIT", "MIN_UNIT", "UNIT_PRE", "制剂单位"],
  },
  {
    key: "dose",
    label: "制剂剂量",
    required: false,
    group: "基本信息",
    hint: "耗材可不填",
    aliases: ["DOSE", "DOSAGE", "JL", "剂量"],
  },
  {
    key: "unitDose",
    label: "剂量单位",
    required: false,
    group: "基本信息",
    hint: "耗材可不填",
    aliases: ["DOSE_UNIT", "UNIT_DOSE", "JLDW", "剂量单位"],
  },
  {
    key: "spec",
    label: "制剂规格",
    required: false,
    group: "基本信息",
    hint: "为空时自动按剂量生成",
    aliases: ["SPEC", "DRUG_SPEC", "GG", "规格"],
  },
  {
    key: "dftUsage",
    label: "默认给药方法编码",
    required: false,
    group: "用药规则",
    hint: "草药、耗材可不填",
    aliases: ["USAGE_CODE", "DFT_USAGE", "GYFF", "用法"],
  },
  {
    key: "dftFreq",
    label: "默认频次编码",
    required: false,
    group: "用药规则",
    hint: "草药、耗材可不填",
    aliases: ["FREQ_CODE", "DFT_FREQ", "PCDM", "频次"],
  },
  {
    key: "dftDoseOnce",
    label: "默认一次用量",
    required: false,
    group: "用药规则",
    hint: "原模板“一次用量”，草药未填时使用制剂剂量",
    aliases: ["DOSE_ONCE", "DFT_DOSE_ONCE", "YCYL", "一次用量"],
  },
  {
    key: "sdRound",
    label: "取整策略编码",
    required: false,
    group: "用药规则",
    hint: "需映射为新系统取整策略字典编码",
    aliases: ["ROUND_CODE", "SD_ROUND", "QZCL", "取整策略"],
  },
  {
    key: "sdDps",
    label: "发药方式编码",
    required: false,
    group: "用药规则",
    hint: "需映射为新系统发药方式字典编码",
    aliases: ["DISPENSE_CODE", "SD_DPS", "FYFS", "发药方式"],
  },
  {
    key: "idFac",
    label: "生产厂家主键",
    required: false,
    group: "厂家商品",
    hint: "已有厂家对照时优先使用",
    aliases: ["FACTORY_ID", "ID_FAC", "CJID"],
  },
  {
    key: "naFac",
    label: "生产厂家名称",
    required: false,
    group: "厂家商品",
    hint: "厂家主键为空时用于精确匹配",
    aliases: ["FACTORY_NAME", "MANUFACTURER", "SCCJ", "生产厂家"],
  },
  {
    key: "naMedPro",
    label: "商品名",
    required: false,
    group: "厂家商品",
    hint: "为空时使用通用名",
    aliases: ["PRODUCT_NAME", "BRAND_NAME", "SPM", "商品名"],
  },
  {
    key: "unitSale",
    label: "零售包装单位",
    required: true,
    group: "包装价格",
    hint: "例如盒、瓶、支",
    aliases: ["SALE_UNIT", "PACK_UNIT", "UNIT_SALE", "包装单位"],
  },
  {
    key: "unitSaleFactor",
    label: "包装系数",
    required: true,
    group: "包装价格",
    hint: "必须为正整数",
    aliases: ["PACK_FACTOR", "UNIT_FACTOR", "BZXS", "包装系数"],
  },
  {
    key: "specSale",
    label: "零售包装规格",
    required: false,
    group: "包装价格",
    hint: "为空时自动生成",
    aliases: ["SALE_SPEC", "PACK_SPEC", "SPEC_SALE", "包装规格"],
  },
  {
    key: "pricePur",
    label: "进货价格",
    required: true,
    group: "包装价格",
    hint: "允许为0，不允许负数",
    aliases: ["BUY_PRICE", "PURCHASE_PRICE", "PRICE_PUR", "进货价"],
  },
  {
    key: "priceSale",
    label: "零售价格",
    required: true,
    group: "包装价格",
    hint: "允许为0，不允许负数",
    aliases: ["RETAIL_PRICE", "SALE_PRICE", "PRICE_SALE", "零售价"],
  },
  {
    key: "cdAppr",
    label: "批准文号",
    required: false,
    group: "监管编码",
    hint: "药品批准文号",
    aliases: ["APPROVAL_NO", "LICENSE_NO", "CD_APPR", "批准文号"],
  },
  {
    key: "cdBar",
    label: "条形码",
    required: false,
    group: "监管编码",
    hint: "商品条形码",
    aliases: ["BARCODE", "CD_BAR", "条形码"],
  },
  {
    key: "cdMedPro",
    label: "货品码",
    required: false,
    group: "监管编码",
    hint: "三方商品编码",
    aliases: ["DRUG_CODE", "ITEM_CODE", "CD_MED_PRO", "货品码"],
  },
  {
    key: "sdBasMed",
    label: "基药类型编码",
    required: false,
    group: "监管属性",
    hint: "需映射为新系统基药类型字典编码",
    aliases: ["BASIC_DRUG_TYPE", "SD_BAS_MED", "JYLX", "基药类型"],
  },
  {
    key: "fgMedRx",
    label: "处方药标志",
    required: false,
    group: "监管属性",
    hint: "统一转换为0否、1是",
    aliases: ["RX_FLAG", "FG_MED_RX", "CFYP", "处方药品"],
  },
  {
    key: "sdChrgitmLv",
    label: "药品档次编码",
    required: false,
    group: "监管属性",
    hint: "原模板“档次”，按新系统字典编码接入",
    aliases: ["CHARGE_LEVEL", "SD_CHRGITM_LV", "YPDC", "档次"],
  },
  {
    key: "sdStorage",
    label: "药品储藏编码",
    required: false,
    group: "监管属性",
    hint: "需映射为新系统储藏字典编码",
    aliases: ["STORAGE_CODE", "SD_STORAGE", "YPCC", "药品储藏"],
  },
  {
    key: "sdSpeMed",
    label: "特殊药品编码",
    required: false,
    group: "监管属性",
    hint: "需映射为新系统特殊药品字典编码",
    aliases: ["SPECIAL_DRUG_TYPE", "SD_SPE_MED", "TSYP", "特殊药品"],
  },
  {
    key: "limitAntiDay",
    label: "一日限量",
    required: false,
    group: "监管属性",
    hint: "不得小于0",
    aliases: ["DAILY_LIMIT", "LIMIT_ANTI_DAY", "YRXL", "一日限量"],
  },
  {
    key: "fgCollPur",
    label: "集采标志",
    required: false,
    group: "商品属性",
    hint: "统一转换为0否、1是",
    aliases: ["COLLECTIVE_PURCHASE", "FG_COLL_PUR", "JCBS", "集采"],
  },
  {
    key: "fgImport",
    label: "进口药品标志",
    required: false,
    group: "商品属性",
    hint: "统一转换为0否、1是",
    aliases: ["IMPORT_FLAG", "FG_IMPORT", "JKYP", "进口药品"],
  },
];

const databaseKinds = {
  mysql: {
    label: "MySQL",
    port: 3306,
    protocol: "原生 Rust 驱动",
    defaultDriver: "",
  },
  oracle: {
    label: "Oracle",
    port: 1521,
    protocol: "Instant Client ODBC 19c",
    defaultDriver: "Oracle 19 ODBC driver",
  },
  dameng: {
    label: "达梦 DM8",
    port: 5236,
    protocol: "官方通用 ODBC 驱动",
    defaultDriver: "DM8 ODBC DRIVER",
  },
  opengauss: {
    label: "Gauss / openGauss",
    port: 5432,
    protocol: "官方通用 ODBC 驱动",
    defaultDriver: "openGauss",
  },
  kingbase: {
    label: "人大金仓 KingbaseES",
    port: 54321,
    protocol: "官方通用 ODBC 驱动",
    defaultDriver: "KingbaseES 8.6 ODBC Driver",
  },
  postgresql: {
    label: "PostgreSQL",
    port: 5432,
    protocol: "psqlODBC 通用驱动",
    defaultDriver: "PostgreSQL Unicode(x64)",
  },
};

const driverKeywords = {
  oracle: ["oracle"],
  dameng: ["dm8", "dameng", "达梦"],
  opengauss: ["opengauss", "gauss"],
  kingbase: ["kingbase", "金仓"],
  postgresql: ["postgresql", "psql"],
};

const initialProfile = {
  kind: "mysql",
  host: "127.0.0.1",
  port: 3306,
  database: "",
  username: "",
  password: "",
  schema: "",
  serviceName: "",
  driver: "",
  connectionString: "",
};

function statusMeta(status) {
  return (
    {
      VALIDATED: ["可迁移", "ready"],
      INVALID: ["校验失败", "danger"],
      RUNNING: ["执行中", "running"],
      SUCCESS: ["成功", "success"],
      FAILED: ["失败", "danger"],
      PARTIAL: ["部分完成", "warning"],
      SKIPPED: ["已跳过", "muted"],
    }[status] || [status || "未开始", "muted"]
  );
}

function Stepper({ active }) {
  return (
    <nav className="stepper" aria-label="迁移进度">
      {steps.map((step, index) => (
        <div
          className={`step ${index < active ? "step--done" : ""} ${index === active ? "step--active" : ""}`}
          key={step}
        >
          <div className="step__node">
            {index < active ? <Check size={16} weight="bold" /> : index + 1}
          </div>
          <span>{step}</span>
          {index < steps.length - 1 && <div className="step__line" />}
        </div>
      ))}
    </nav>
  );
}

function Header({ runtime }) {
  return (
    <header className="topbar">
      <div className="brand">
        <FirstAidKit className="brand-mark" size={34} weight="duotone" />
        <span className="brand__title">数据迁移助手</span>
      </div>
      <div className="topbar__divider" />
      <span className="topbar__section">药品基础数据</span>
      <span className="desktop-chip">
        {runtime === "tauri-rust" ? "桌面版 · 本地运行" : "交互预览"}
      </span>
      <div className="topbar__spacer" />
      <div className="save-status">
        <ShieldCheck size={18} weight="fill" />
        审计记录保存在本机
      </div>
      <div className="topbar__time">
        <Clock size={18} />
        {new Date().toLocaleDateString("zh-CN")}
      </div>
      <button className="user-menu">
        <UserCircle size={27} weight="fill" />
        <span>实施人员</span>
      </button>
    </header>
  );
}

function Field({ label, children, wide }) {
  return (
    <label className={`form-field ${wide ? "form-field--wide" : ""}`}>
      <span>{label}</span>
      {children}
    </label>
  );
}

function ConnectionForm({
  value,
  onChange,
  title,
  drivers = [],
  driverPacks = [],
}) {
  const set = (key, next) =>
    onChange({ ...value, [key]: key === "port" ? Number(next) : next });
  const kind = databaseKinds[value.kind] || databaseKinds.mysql;
  const driverPack = driverPacks.find(
    (pack) => pack.databaseKind === value.kind,
  );
  const changeKind = (nextKind) => {
    const next = databaseKinds[nextKind];
    const keywords = driverKeywords[nextKind] || [];
    const recommended = drivers.find((driver) =>
      keywords.some((keyword) => driver.toLowerCase().includes(keyword)),
    );
    onChange({
      ...value,
      kind: nextKind,
      port: next.port,
      driver:
        nextKind === "mysql"
          ? ""
          : recommended ||
            driverPacks.find((pack) => pack.databaseKind === nextKind)
              ?.defaultDriver ||
            next.defaultDriver,
      connectionString: "",
    });
  };
  return (
    <div className="connection-form">
      <div className="section-title">
        <Database size={21} weight="duotone" />
        <div>
          <strong>{title}</strong>
          <small>
            {kind.label} · {kind.protocol}
          </small>
        </div>
      </div>
      {driverPack && value.kind !== "mysql" && (
        <div className={`driver-preset driver-preset--${driverPack.state}`}>
          <div className="driver-preset__topline">
            <span className="driver-preset__name">
              <ShieldCheck size={17} weight="fill" />
              {driverPack.title} · {driverPack.version}
            </span>
            <span className="driver-preset__status">
              {driverPack.state === "installed"
                ? "本机已就绪"
                : driverPack.state === "bundled"
                  ? "应用内置"
                  : "驱动文件未安装"}
            </span>
          </div>
          <p>
            {driverPack.detectedDriver
              ? `已识别驱动：${driverPack.detectedDriver}`
              : `当前只有连接配置，安装或导入驱动文件后才能连接。${driverPack.licenseNote}`}
          </p>
        </div>
      )}
      <div className="form-grid">
        <Field label="数据库类型">
          <select
            value={value.kind}
            onChange={(event) => changeKind(event.target.value)}
          >
            {Object.entries(databaseKinds).map(([key, item]) => (
              <option value={key} key={key}>
                {item.label}
              </option>
            ))}
          </select>
        </Field>
        <Field label="主机地址">
          <input
            value={value.host}
            onChange={(event) => set("host", event.target.value)}
            placeholder="例如 192.168.1.20"
          />
        </Field>
        <Field label="端口">
          <input
            type="number"
            value={value.port}
            onChange={(event) => set("port", event.target.value)}
          />
        </Field>
        <Field label="数据库名">
          <input
            value={value.database}
            onChange={(event) => set("database", event.target.value)}
            placeholder={
              value.kind === "oracle" ? "数据库或 PDB 名称" : "数据库名称"
            }
          />
        </Field>
        {value.kind === "oracle" && (
          <Field label="Service Name">
            <input
              value={value.serviceName}
              onChange={(event) => set("serviceName", event.target.value)}
              placeholder="例如 ORCLPDB1"
            />
          </Field>
        )}
        {value.kind !== "mysql" && (
          <Field label="Schema / 模式">
            <input
              value={value.schema}
              onChange={(event) => set("schema", event.target.value)}
              placeholder="不填则读取当前账号可见表"
            />
          </Field>
        )}
        <Field label={title.includes("老系统") ? "只读账号" : "写入账号"}>
          <input
            value={value.username}
            onChange={(event) => set("username", event.target.value)}
            autoComplete="off"
          />
        </Field>
        <Field label="密码">
          <div className="password-input">
            <LockKey size={17} />
            <input
              type="password"
              value={value.password}
              onChange={(event) => set("password", event.target.value)}
              autoComplete="new-password"
            />
          </div>
        </Field>
        {value.kind !== "mysql" && (
          <>
            <Field label="本机数据库驱动" wide>
              <input
                list="database-driver-list"
                value={value.driver}
                onChange={(event) => set("driver", event.target.value)}
                placeholder={
                  drivers.length
                    ? `推荐：${kind.defaultDriver}`
                    : `已预置：${kind.defaultDriver}`
                }
              />
              <datalist id="database-driver-list">
                {drivers.map((driver) => (
                  <option value={driver} key={driver} />
                ))}
              </datalist>
              <small>
                {drivers.length
                  ? `已检测到 ${drivers.length} 个 ODBC 驱动`
                  : "未检测到本机驱动；连接测试时会给出安装或导入提示"}
              </small>
            </Field>
            <Field label="高级连接串（可选）" wide>
              <textarea
                rows={2}
                value={value.connectionString}
                onChange={(event) =>
                  set("connectionString", event.target.value)
                }
                placeholder="${HOST}、${PORT}、${DATABASE}、${SERVICE}、${SCHEMA}、${USER}、${PASSWORD}"
              />
            </Field>
          </>
        )}
      </div>
      <p className="security-note">
        <ShieldCheck size={16} />
        密码仅在当前运行期间保存在内存，不写入迁移配置或审计日志。
      </p>
    </div>
  );
}

function DataTable({ columns, rows, maxRows = 6 }) {
  const shown = columns.slice(0, 7);
  return (
    <div className="data-table">
      <div
        className="data-table__row data-table__head"
        style={{
          gridTemplateColumns: `repeat(${shown.length}, minmax(140px, 1fr))`,
        }}
      >
        {shown.map((column) => (
          <span key={column}>{column}</span>
        ))}
      </div>
      {rows.slice(0, maxRows).map((row, index) => (
        <div
          className="data-table__row"
          style={{
            gridTemplateColumns: `repeat(${shown.length}, minmax(140px, 1fr))`,
          }}
          key={index}
        >
          {shown.map((column) => (
            <span
              title={`${row[column] ?? ""}`}
              key={column}
            >{`${row[column] ?? "—"}`}</span>
          ))}
        </div>
      ))}
    </div>
  );
}

function SummaryCards({ batch }) {
  if (!batch) return null;
  const cards = [
    ["总行数", batch.totalCount, "neutral"],
    ["可迁移", batch.validCount, "ready"],
    ["成功", batch.successCount, "success"],
    ["跳过", batch.skipCount, "muted"],
    ["失败", batch.failCount, "danger"],
  ];
  return (
    <div className="summary-cards">
      {cards.map(([label, value, tone]) => (
        <div className={`summary-card summary-card--${tone}`} key={label}>
          <span>{label}</span>
          <strong>{value}</strong>
        </div>
      ))}
    </div>
  );
}

function parseCsv(text) {
  const rows = [];
  let row = [],
    cell = "",
    quoted = false;
  for (let i = 0; i < text.length; i += 1) {
    const char = text[i],
      next = text[i + 1];
    if (char === '"' && quoted && next === '"') {
      cell += '"';
      i += 1;
    } else if (char === '"') quoted = !quoted;
    else if (char === "," && !quoted) {
      row.push(cell);
      cell = "";
    } else if ((char === "\n" || char === "\r") && !quoted) {
      if (char === "\r" && next === "\n") i += 1;
      row.push(cell);
      if (row.some((item) => item !== "")) rows.push(row);
      row = [];
      cell = "";
    } else cell += char;
  }
  row.push(cell);
  if (row.some((item) => item !== "")) rows.push(row);
  const [headers = [], ...data] = rows;
  return data.map((values) =>
    Object.fromEntries(
      headers.map((header, index) => [header.trim(), values[index] ?? ""]),
    ),
  );
}

export function App() {
  const fileInput = useRef(null);
  const [runtime, setRuntime] = useState(
    isDesktop ? "tauri-rust" : "browser-preview",
  );
  const [step, setStep] = useState(0);
  const [sourceMode, setSourceMode] = useState("file");
  const [sourceProfile, setSourceProfile] = useState(initialProfile);
  const [targetProfile, setTargetProfile] = useState(initialProfile);
  const [query, setQuery] = useState("SELECT * FROM T_DRUG_INFO");
  const [sourceName, setSourceName] = useState("尚未选择数据源");
  const [rows, setRows] = useState([]);
  const [columns, setColumns] = useState([]);
  const [sourceKey, setSourceKey] = useState("");
  const [mapping, setMapping] = useState({});
  const [rules, setRules] = useState({});
  const [fieldIndex, setFieldIndex] = useState(0);
  const [expert, setExpert] = useState(false);
  const [allowCreateFactory, setAllowCreateFactory] = useState(false);
  const [conflictStrategy, setConflictStrategy] = useState("REUSE");
  const [batchDetail, setBatchDetail] = useState(null);
  const [tenantId, setTenantId] = useState("");
  const [operatorId, setOperatorId] = useState("");
  const [organizationId, setOrganizationId] = useState("");
  const [activeResultTab, setActiveResultTab] = useState("rows");
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState(null);
  const [databaseDrivers, setDatabaseDrivers] = useState([]);
  const [driverPacks, setDriverPacks] = useState([]);

  useEffect(() => {
    command("app_health")
      .then((health) => setRuntime(health.runtime))
      .catch(() => {});
    command("list_database_drivers")
      .then(setDatabaseDrivers)
      .catch(() => setDatabaseDrivers([]));
    command("list_driver_packs")
      .then(setDriverPacks)
      .catch(() => setDriverPacks([]));
  }, []);

  const currentField = targetFields[fieldIndex];
  const mappedCount = targetFields.filter(
    (field) => mapping[field.key] || rules[field.key]?.defaultValue,
  ).length;
  const suggestions = useMemo(() => {
    if (!currentField) return [];
    return [...columns]
      .map((column) => ({ column, score: matchScore(column, currentField) }))
      .sort((a, b) => b.score - a.score)
      .slice(0, 3);
  }, [columns, currentField]);

  const notify = (message, tone = "success") => {
    setNotice({ message, tone });
    window.setTimeout(() => setNotice(null), 3000);
  };
  const fail = (error) =>
    notify(
      typeof error === "string" ? error : error?.message || `${error}`,
      "danger",
    );

  function acceptData(data, name) {
    if (!data.length) return fail("文件中没有可识别的数据行");
    const detectedColumns = Object.keys(data[0]);
    setRows(data);
    setColumns(detectedColumns);
    setSourceName(name);
    setSourceKey(detectedColumns[0] || "");
    const auto = {};
    targetFields.forEach((field) => {
      const best = detectedColumns
        .map((column) => ({ column, score: matchScore(column, field) }))
        .sort((a, b) => b.score - a.score)[0];
      if (best?.score >= 55) auto[field.key] = best.column;
    });
    setMapping(auto);
    setStep(1);
    notify(`已读取 ${data.length} 行、${detectedColumns.length} 个字段`);
  }

  async function loadFile(file) {
    try {
      const text = await file.text();
      const data = file.name.toLowerCase().endsWith(".json")
        ? JSON.parse(text)
        : parseCsv(text);
      acceptData(Array.isArray(data) ? data : data.rows || [], file.name);
    } catch (error) {
      fail(`文件解析失败：${error.message}`);
    }
  }

  async function loadDatabase() {
    setBusy("source");
    try {
      const checked = await command("test_database_connection", {
        profile: sourceProfile,
      });
      const preview = await command("preview_source", {
        request: { connection: sourceProfile, query, limit: 500 },
      });
      acceptData(
        preview.rows,
        `${databaseKinds[sourceProfile.kind]?.label || sourceProfile.kind} · ${sourceProfile.database} · 自定义只读查询`,
      );
      notify(`${checked.message}，已预览 ${preview.rows.length} 行`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function testSourceConnection() {
    setBusy("source-test");
    try {
      const checked = await command("test_database_connection", {
        profile: sourceProfile,
      });
      notify(`${checked.message} · ${checked.latencyMs}ms`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function beginMapping() {
    setFieldIndex(0);
    setStep(2);
  }

  function fieldRulePreview(field) {
    const sourceField = mapping[field.key];
    if (!sourceField || !rows.length) return "";
    const original = rows.find(
      (row) => `${row[sourceField] ?? ""}`.trim() !== "",
    )?.[sourceField];
    if (original === undefined) return "";
    const rule = rules[field.key] || {};
    const converted = applyFieldRule(original, {
      ...rule,
      valueMappings: parseValueMappings(rule.valueMappingsText).mappings,
    });
    return `样例：${original} → ${converted ?? "空值"}`;
  }

  async function prepareBatch() {
    const invalidMapping = targetFields
      .map((field) => ({
        field,
        parsed: parseValueMappings(rules[field.key]?.valueMappingsText),
      }))
      .find(({ parsed }) => parsed.invalidLines.length);
    if (invalidMapping) {
      return fail(
        `${invalidMapping.field.label}的值映射第 ${invalidMapping.parsed.invalidLines.join("、")} 行格式不正确，请使用“旧值 = 新值”`,
      );
    }
    const mappings = targetFields
      .filter((field) => mapping[field.key] || rules[field.key]?.defaultValue)
      .map((field) => {
        const rule = rules[field.key] || {};
        return {
          sourceField: mapping[field.key] || "",
          targetField: field.key,
          transform: rule.transform || "TRIM",
          defaultValue: rule.defaultValue || "",
          valueMappings: parseValueMappings(rule.valueMappingsText).mappings,
          valueMappingCaseInsensitive:
            rule.valueMappingCaseInsensitive || false,
        };
      });
    const preparedRows = rows.map((row) => ({
      ...row,
      _sourceKey: row[sourceKey] ?? "",
    }));
    setBusy("prepare");
    try {
      const detail = await command("prepare_migration_batch", {
        request: {
          batchName: `${sourceName}-药品迁移`,
          sourceType: sourceMode === "database" ? "DATABASE" : "FILE",
          sourceName,
          sourceDescription: query,
          conflictStrategy,
          allowCreateFactory,
          idempotencyKey: objectId(),
          mappings,
          rows: preparedRows,
        },
      });
      setBatchDetail(detail);
      notify(`校验完成：${detail.batch.validCount} 行可迁移`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function testTarget() {
    setBusy("target-test");
    try {
      const result = await command("test_database_connection", {
        profile: targetProfile,
      });
      const readiness = await command("inspect_target_schema", {
        profile: targetProfile,
      });
      notify(
        `${result.message} · 已核对 ${readiness.checkedTables.length} 张药品表 · ${result.latencyMs}ms`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function execute(failedOnly = false) {
    if (!batchDetail) return;
    if (!tenantId.trim() || !operatorId.trim())
      return fail("请填写新系统租户 ID 和操作人 ID");
    setBusy(failedOnly ? "retry" : "execute");
    try {
      const detail = await command("execute_migration_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
          failedOnly,
          tenantId,
          operatorId,
          organizationId,
        },
      });
      setBatchDetail(detail);
      setStep(4);
      notify(failedOnly ? "失败行重试完成" : "迁移执行完成");
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  return (
    <div className="app-shell">
      <Header runtime={runtime} />
      <Stepper active={step} />
      <main className="workspace">
        {step === 0 && (
          <section className="screen source-screen">
            <div className="screen-heading">
              <span className="eyebrow">第 1 步 · 数据来源</span>
              <h1>三方数据从哪里来？</h1>
              <p>
                可以直接连接老数据库，也可以导入 CSV / JSON
                文件。数据库模式只执行只读查询。
              </p>
            </div>
            <div className="source-tabs">
              <button
                className={sourceMode === "file" ? "active" : ""}
                onClick={() => setSourceMode("file")}
              >
                <FileCsv size={22} />
                文件导入
              </button>
              <button
                className={sourceMode === "database" ? "active" : ""}
                onClick={() => setSourceMode("database")}
              >
                <Database size={22} />
                数据库直连
              </button>
            </div>
            {sourceMode === "file" ? (
              <div
                className="upload-zone"
                onClick={() => fileInput.current?.click()}
              >
                <input
                  ref={fileInput}
                  type="file"
                  accept=".csv,.json"
                  hidden
                  onChange={(event) =>
                    event.target.files?.[0] && loadFile(event.target.files[0])
                  }
                />
                <UploadSimple size={44} weight="duotone" />
                <h2>选择三方数据文件</h2>
                <p>支持 UTF-8 CSV、JSON；首行作为字段名，单批最多 10,000 行</p>
                <button className="button button--primary">选择文件</button>
              </div>
            ) : (
              <div className="source-db-layout">
                <ConnectionForm
                  value={sourceProfile}
                  onChange={setSourceProfile}
                  title="老系统只读连接"
                  drivers={databaseDrivers}
                  driverPacks={driverPacks}
                />
                <Field label="读取数据的 SQL" wide>
                  <textarea
                    value={query}
                    onChange={(event) => setQuery(event.target.value)}
                    rows={5}
                  />
                  <small>
                    仅允许 SELECT / WITH；如只验证驱动，可先点击“仅测试连接”。
                  </small>
                </Field>
                <div className="source-actions">
                  <button
                    className="button button--secondary"
                    disabled={["source", "source-test"].includes(busy)}
                    onClick={testSourceConnection}
                  >
                    <Plugs size={20} />
                    {busy === "source-test" ? "正在测试…" : "仅测试连接"}
                  </button>
                  <button
                    className="button button--primary"
                    disabled={["source", "source-test"].includes(busy)}
                    onClick={loadDatabase}
                  >
                    <Rows size={20} />
                    {busy === "source" ? "正在读取…" : "连接并读取预览"}
                  </button>
                </div>
              </div>
            )}
            <div className="demo-line">
              <span>还没有数据？</span>
              <button
                onClick={() =>
                  acceptData(demoRows(), "内置演示数据 · T_DRUG_INFO")
                }
              >
                使用示例药品数据体验完整流程 <ArrowRight size={16} />
              </button>
            </div>
          </section>
        )}

        {step === 1 && (
          <section className="screen">
            <div className="screen-heading screen-heading--row">
              <div>
                <span className="eyebrow">第 2 步 · 识别数据</span>
                <h1>确认要迁移的数据范围</h1>
                <p>
                  系统已读取数据。请选择能代表三方记录唯一性的字段，它只用于追溯，不会作为新表主键。
                </p>
              </div>
              <div className="source-summary">
                <Database size={24} />
                <div>
                  <small>当前来源</small>
                  <strong>{sourceName}</strong>
                </div>
              </div>
            </div>
            <div className="recognition-grid">
              <div className="stat-panel">
                <div>
                  <span>读取行数</span>
                  <strong>{rows.length}</strong>
                </div>
                <div>
                  <span>来源字段</span>
                  <strong>{columns.length}</strong>
                </div>
                <div>
                  <span>空记录</span>
                  <strong>
                    {
                      rows.filter((row) =>
                        Object.values(row).every(
                          (value) => `${value ?? ""}`.trim() === "",
                        ),
                      ).length
                    }
                  </strong>
                </div>
              </div>
              <Field label="三方记录唯一标识">
                <select
                  value={sourceKey}
                  onChange={(event) => setSourceKey(event.target.value)}
                >
                  {columns.map((column) => (
                    <option key={column}>{column}</option>
                  ))}
                </select>
                <small>
                  例如药品编码或三方主键。它会写入本地迁移日志，便于反查老系统。
                </small>
              </Field>
            </div>
            <div className="table-heading">
              <div>
                <Table size={20} />
                <strong>数据预览</strong>
                <span>前 {Math.min(6, rows.length)} 行</span>
              </div>
              <span className="read-only">
                <LockKey size={15} />
                只读
              </span>
            </div>
            <DataTable columns={columns} rows={rows} />
            <div className="screen-actions">
              <button
                className="button button--secondary"
                onClick={() => setStep(0)}
              >
                <ArrowLeft />
                重新选择
              </button>
              <button className="button button--primary" onClick={beginMapping}>
                开始匹配新系统字段
                <ArrowRight />
              </button>
            </div>
          </section>
        )}

        {step === 2 && (
          <section className="screen mapping-screen">
            <div className="mapping-top">
              <div className="mapping-progress">
                <Rows size={26} weight="duotone" />
                <div>
                  <strong>
                    已识别 {mappedCount} / {targetFields.length} 个目标字段
                  </strong>
                  <div className="progress-track">
                    <span
                      style={{
                        width: `${(mappedCount / targetFields.length) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              </div>
              <button className="expert-switch" onClick={() => setExpert(true)}>
                <Code size={19} />
                专业映射模式
              </button>
            </div>
            <div className="source-line">
              <Database size={20} />
              <span>来源：</span>
              <strong>{sourceName}</strong>
              <span className="source-chip">唯一标识 {sourceKey}</span>
            </div>
            <div className="question-copy">
              <span className="field-group">{currentField.group}</span>
              <h1>
                老系统里，哪个字段代表“{currentField.label}”？
                {currentField.required && <em>必填</em>}
              </h1>
              <p>{currentField.hint}</p>
            </div>
            <div className="suggestion-heading">
              <span>智能推荐</span>
              <small>基于字段名和常用医院数据结构</small>
              <Info size={15} />
            </div>
            <div className="suggestions">
              {suggestions.map((item, index) => (
                <button
                  className={`suggestion-row ${mapping[currentField.key] === item.column ? "suggestion-row--selected" : ""}`}
                  onClick={() =>
                    setMapping((current) => ({
                      ...current,
                      [currentField.key]: item.column,
                    }))
                  }
                  key={item.column}
                >
                  <span className="radio">
                    {mapping[currentField.key] === item.column && <i />}
                  </span>
                  <strong>{item.column}</strong>
                  <span>
                    <small>样例</small>
                    {`${rows[0]?.[item.column] ?? "—"}`}
                  </span>
                  <b
                    className={
                      index === 0
                        ? "badge badge--good"
                        : "badge badge--possible"
                    }
                  >
                    {index === 0 ? "推荐" : "可能匹配"}
                  </b>
                  <strong className="score">{item.score}%</strong>
                </button>
              ))}
            </div>
            <div className="inline-select">
              <span>没有合适的推荐？</span>
              <select
                value={mapping[currentField.key] || ""}
                onChange={(event) =>
                  setMapping((current) => ({
                    ...current,
                    [currentField.key]: event.target.value,
                  }))
                }
              >
                <option value="">不映射 / 稍后使用默认值</option>
                {columns.map((column) => (
                  <option key={column}>{column}</option>
                ))}
              </select>
            </div>
            <div className="preview-card">
              <div>
                <LinkSimple size={17} />
                <strong>即时预览</strong>
                <span>
                  {mapping[currentField.key] || "未选择来源字段"} →{" "}
                  {currentField.key}
                </span>
              </div>
              <div className="preview-values">
                {rows.slice(0, 3).map((row, index) => (
                  <span key={index}>
                    {mapping[currentField.key]
                      ? `${row[mapping[currentField.key]] ?? "空值"}`
                      : "—"}
                  </span>
                ))}
              </div>
            </div>
            <div className="screen-actions">
              <button
                className="button button--secondary"
                onClick={() =>
                  fieldIndex ? setFieldIndex(fieldIndex - 1) : setStep(1)
                }
              >
                <ArrowLeft />
                上一个
              </button>
              <div>
                <button
                  className="button button--ghost"
                  onClick={() =>
                    setMapping((current) => ({
                      ...current,
                      [currentField.key]: "",
                    }))
                  }
                >
                  此字段先不匹配
                </button>
                <button
                  className="button button--primary"
                  onClick={() => {
                    if (fieldIndex < targetFields.length - 1)
                      setFieldIndex(fieldIndex + 1);
                    else setStep(3);
                  }}
                >
                  {fieldIndex < targetFields.length - 1
                    ? `确认，继续匹配${targetFields[fieldIndex + 1].label}`
                    : "完成映射，进入校验"}
                  <ArrowRight />
                </button>
              </div>
            </div>
          </section>
        )}

        {step === 3 && (
          <section className="screen">
            <div className="screen-heading">
              <span className="eyebrow">第 4 步 · 校验与修正</span>
              <h1>先试迁移，再决定是否正式写入</h1>
              <p>
                此阶段只把原始数据、转换结果和问题写入本地暂存库，不会修改新系统业务表。
              </p>
            </div>
            <div className="rules-layout">
              <div className="rules-card">
                <div className="section-title">
                  <Gear size={21} />
                  <div>
                    <strong>迁移策略</strong>
                    <small>这些设置会随批次进入审计记录</small>
                  </div>
                </div>
                <label className="option-row">
                  <span>
                    <strong>目标重复时</strong>
                    <small>
                      基础药品按“通用名 + 类型 + 规格 + 可见范围”复用；商品优先按货品码，
                      再按“厂家 + 商品名 + 销售规格”判重
                    </small>
                  </span>
                  <select
                    value={conflictStrategy}
                    onChange={(event) =>
                      setConflictStrategy(event.target.value)
                    }
                  >
                    <option value="REUSE">复用并幂等跳过（推荐）</option>
                    <option value="FAIL">发现重复即报错</option>
                  </select>
                </label>
                <label className="option-row">
                  <span>
                    <strong>未匹配到生产厂家</strong>
                    <small>关闭时该行失败，不会静默制造厂家脏数据</small>
                  </span>
                  <input
                    className="switch"
                    type="checkbox"
                    checked={allowCreateFactory}
                    onChange={(event) =>
                      setAllowCreateFactory(event.target.checked)
                    }
                  />
                </label>
              </div>
              <div className="rules-card">
                <div className="section-title">
                  <FloppyDisk size={21} />
                  <div>
                    <strong>字段转换</strong>
                    <small>支持清洗、大小写、数值日期、默认值和值字典映射</small>
                  </div>
                </div>
                <div className="rule-list">
                  {targetFields
                    .filter(
                      (field) =>
                        mapping[field.key] ||
                        field.required ||
                        rules[field.key]?.defaultValue ||
                        rules[field.key]?.valueMappingsText,
                    )
                    .map((field) => (
                      <div className="rule-row" key={field.key}>
                        <span>
                          <strong>
                            {field.label}
                            {field.required && <em>*</em>}
                          </strong>
                          <small>
                            {mapping[field.key]
                              ? `来自 ${mapping[field.key]}${fieldRulePreview(field) ? `；${fieldRulePreview(field)}` : ""}`
                              : "尚未匹配来源字段"}
                          </small>
                        </span>
                        <select
                          value={rules[field.key]?.transform || "TRIM"}
                          onChange={(event) =>
                            setRules((current) => ({
                              ...current,
                              [field.key]: {
                                ...current[field.key],
                                transform: event.target.value,
                              },
                            }))
                          }
                        >
                          <option value="TRIM">清理首尾空格</option>
                          <option value="COLLAPSE_WHITESPACE">合并连续空格</option>
                          <option value="REMOVE_WHITESPACE">移除全部空格</option>
                          <option value="INTEGER">转为整数</option>
                          <option value="DECIMAL">转为数字</option>
                          <option value="BOOLEAN_01">是/否转 1/0</option>
                          <option value="UPPER">转大写</option>
                          <option value="LOWER">转小写</option>
                          <option value="DATE_YYYY_MM_DD">日期转 YYYY-MM-DD</option>
                        </select>
                        <input
                          placeholder="缺失时使用默认值"
                          value={rules[field.key]?.defaultValue || ""}
                          onChange={(event) =>
                            setRules((current) => ({
                              ...current,
                              [field.key]: {
                                ...current[field.key],
                                defaultValue: event.target.value,
                              },
                            }))
                          }
                        />
                        <details className="value-mapping-editor">
                          <summary>
                            <span>字段值映射（旧值 → 新值）</span>
                            <small>
                              {Object.keys(
                                parseValueMappings(
                                  rules[field.key]?.valueMappingsText,
                                ).mappings,
                              ).length || "未配置"}
                            </small>
                          </summary>
                          <div>
                            <textarea
                              rows={4}
                              placeholder={"西药 = 1\n中成药 = 2\n停用 = 0"}
                              value={rules[field.key]?.valueMappingsText || ""}
                              onChange={(event) =>
                                setRules((current) => ({
                                  ...current,
                                  [field.key]: {
                                    ...current[field.key],
                                    valueMappingsText: event.target.value,
                                  },
                                }))
                              }
                            />
                            <label>
                              <input
                                type="checkbox"
                                checked={
                                  rules[field.key]
                                    ?.valueMappingCaseInsensitive || false
                                }
                                onChange={(event) =>
                                  setRules((current) => ({
                                    ...current,
                                    [field.key]: {
                                      ...current[field.key],
                                      valueMappingCaseInsensitive:
                                        event.target.checked,
                                    },
                                  }))
                                }
                              />
                              英文字母忽略大小写
                            </label>
                            <small>
                              每行一条，格式为“旧值 = 新值”；先执行值映射，再执行上方格式转换。
                            </small>
                          </div>
                        </details>
                      </div>
                    ))}
                </div>
              </div>
            </div>
            {batchDetail && (
              <>
                <SummaryCards batch={batchDetail.batch} />
                <div className="validation-result">
                  <div className="table-heading">
                    <div>
                      <ListMagnifyingGlass size={20} />
                      <strong>逐行校验结果</strong>
                    </div>
                    <span
                      className={`status-pill status-pill--${statusMeta(batchDetail.batch.status)[1]}`}
                    >
                      {statusMeta(batchDetail.batch.status)[0]}
                    </span>
                  </div>
                  <div className="issue-list">
                    {batchDetail.rows.slice(0, 12).map((row) => (
                      <div key={row.rowId}>
                        <span>第 {row.rowNo} 行</span>
                        <code>{row.sourceKey}</code>
                        <span
                          className={`status-pill status-pill--${statusMeta(row.status)[1]}`}
                        >
                          {statusMeta(row.status)[0]}
                        </span>
                        <p>
                          {row.errorMessage || "字段、类型和条件规则均通过"}
                        </p>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            )}
            <div className="screen-actions">
              <button
                className="button button--secondary"
                onClick={() => setStep(2)}
              >
                <ArrowLeft />
                返回映射
              </button>
              <div>
                {batchDetail && (
                  <button
                    className="button button--secondary"
                    onClick={() => setBatchDetail(null)}
                  >
                    调整后重新校验
                  </button>
                )}
                <button
                  className="button button--primary"
                  disabled={busy === "prepare"}
                  onClick={batchDetail ? () => setStep(4) : prepareBatch}
                >
                  {busy === "prepare"
                    ? "正在校验…"
                    : batchDetail
                      ? "确认结果，配置目标库"
                      : "开始试迁移校验"}
                  <ArrowRight />
                </button>
              </div>
            </div>
          </section>
        )}

        {step === 4 && (
          <section className="screen">
            <div className="screen-heading screen-heading--row">
              <div>
                <span className="eyebrow">第 5 步 · 执行与审计</span>
                <h1>确认目标库，正式写入药品表</h1>
                <p>
                  每行使用独立事务。失败行不会影响成功行，可修正后单独重试。
                </p>
              </div>
              {batchDetail && (
                <span
                  className={`batch-status status-pill--${statusMeta(batchDetail.batch.status)[1]}`}
                >
                  {statusMeta(batchDetail.batch.status)[0]}
                </span>
              )}
            </div>
            <SummaryCards batch={batchDetail?.batch} />
            {batchDetail &&
              !["SUCCESS", "PARTIAL"].includes(batchDetail.batch.status) && (
                <div className="target-card">
                  <ConnectionForm
                    value={targetProfile}
                    onChange={setTargetProfile}
                    title="新系统目标数据库"
                    drivers={databaseDrivers}
                    driverPacks={driverPacks}
                  />
                  <div className="target-context">
                    <Field label="租户 ID">
                      <input
                        value={tenantId}
                        onChange={(event) => setTenantId(event.target.value)}
                        placeholder="hi_bd_* 表的 id_tet"
                      />
                    </Field>
                    <Field label="操作人 ID">
                      <input
                        value={operatorId}
                        onChange={(event) => setOperatorId(event.target.value)}
                        placeholder="写入 insert_user 与审计"
                      />
                    </Field>
                    <Field label="机构 ID（私有药品时）">
                      <input
                        value={organizationId}
                        onChange={(event) =>
                          setOrganizationId(event.target.value)
                        }
                        placeholder="可选"
                      />
                    </Field>
                  </div>
                  <div className="target-actions">
                    <button
                      className="button button--secondary"
                      disabled={busy === "target-test"}
                      onClick={testTarget}
                    >
                      <Plugs />
                      {busy === "target-test" ? "测试中…" : "测试目标库连接"}
                    </button>
                    <button
                      className="button button--danger"
                      disabled={
                        busy === "execute" || !batchDetail.batch.validCount
                      }
                      onClick={() => execute(false)}
                    >
                      <Play weight="fill" />
                      {busy === "execute"
                        ? "正在逐行写入…"
                        : `正式迁移 ${batchDetail.batch.validCount} 行`}
                    </button>
                  </div>
                </div>
              )}{" "}
            {batchDetail && (
              <div className="result-panel">
                <div className="result-tabs">
                  <button
                    className={activeResultTab === "rows" ? "active" : ""}
                    onClick={() => setActiveResultTab("rows")}
                  >
                    迁移明细
                  </button>
                  <button
                    className={activeResultTab === "audit" ? "active" : ""}
                    onClick={() => setActiveResultTab("audit")}
                  >
                    审计日志 <span>{batchDetail.audits.length}</span>
                  </button>
                  <div />
                  {batchDetail.batch.failCount > 0 && (
                    <button
                      className="retry-button"
                      disabled={busy === "retry"}
                      onClick={() => execute(true)}
                    >
                      {busy === "retry" ? "重试中…" : "仅重试失败行"}
                    </button>
                  )}
                </div>
                {activeResultTab === "rows" ? (
                  <div className="result-table">
                    <div className="result-table__row result-table__head">
                      <span>来源行</span>
                      <span>状态</span>
                      <span>药品主键</span>
                      <span>商品主键</span>
                      <span>结果说明</span>
                    </div>
                    {batchDetail.rows.map((row) => (
                      <div className="result-table__row" key={row.rowId}>
                        <span>
                          #{row.rowNo} · {row.sourceKey}
                        </span>
                        <span>
                          <i
                            className={`status-dot status-dot--${statusMeta(row.status)[1]}`}
                          />
                          {statusMeta(row.status)[0]}
                        </span>
                        <code>{row.idMed || "—"}</code>
                        <code>{row.idMedPro || "—"}</code>
                        <span title={row.errorMessage}>
                          {row.errorMessage ||
                            (row.status === "SUCCESS"
                              ? "直接写表完成"
                              : "等待执行")}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="audit-timeline">
                    {batchDetail.audits.map((audit) => (
                      <div key={audit.auditId}>
                        <span
                          className={`audit-icon audit-icon--${audit.result === "FAILED" ? "danger" : "success"}`}
                        >
                          {audit.result === "FAILED" ? <X /> : <Check />}
                        </span>
                        <div>
                          <strong>
                            {audit.operation} · {audit.targetTable}
                          </strong>
                          <p>{audit.message}</p>
                          <small>
                            {new Date(audit.operatedAt).toLocaleString("zh-CN")}{" "}
                            · Trace {audit.traceId}
                          </small>
                        </div>
                        <code>{audit.targetId || "—"}</code>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </section>
        )}
      </main>

      {expert && (
        <>
          <div className="panel-backdrop" onClick={() => setExpert(false)} />
          <aside className="expert-panel">
            <div className="expert-panel__header">
              <div>
                <span className="eyebrow">专业模式</span>
                <h2>全部字段映射</h2>
              </div>
              <button className="icon-button" onClick={() => setExpert(false)}>
                <X />
              </button>
            </div>
            <p>一次检查全部来源与目标字段；修改结果会同步回引导模式。</p>
            <div className="expert-grid">
              <div className="expert-grid__head">
                <span>新系统字段</span>
                <span>三方来源字段</span>
                <span>状态</span>
              </div>
              {targetFields.map((field) => (
                <div key={field.key}>
                  <span>
                    <strong>{field.label}</strong>
                    <code>{field.key}</code>
                  </span>
                  <select
                    value={mapping[field.key] || ""}
                    onChange={(event) =>
                      setMapping((current) => ({
                        ...current,
                        [field.key]: event.target.value,
                      }))
                    }
                  >
                    <option value="">不映射</option>
                    {columns.map((column) => (
                      <option key={column}>{column}</option>
                    ))}
                  </select>
                  <span
                    className={`status-pill status-pill--${mapping[field.key] ? "ready" : field.required ? "danger" : "muted"}`}
                  >
                    {mapping[field.key]
                      ? "已匹配"
                      : field.required
                        ? "待处理"
                        : "可选"}
                  </span>
                </div>
              ))}
            </div>
            <div className="expert-actions">
              <button
                className="button button--secondary"
                onClick={() => setExpert(false)}
              >
                返回引导模式
              </button>
              <button
                className="button button--primary"
                onClick={() => {
                  setExpert(false);
                  setStep(3);
                }}
              >
                保存并进入校验
              </button>
            </div>
          </aside>
        </>
      )}
      {notice && (
        <div className={`toast toast--${notice.tone}`}>
          {notice.tone === "danger" ? (
            <Warning weight="fill" />
          ) : (
            <CheckCircle weight="fill" />
          )}
          {notice.message}
        </div>
      )}
    </div>
  );
}

function matchScore(column, field) {
  const normalized = column
    .toUpperCase()
    .replace(/[^A-Z0-9\u4e00-\u9fa5]/g, "");
  const exact = field.aliases.find(
    (alias) =>
      alias.toUpperCase().replace(/[^A-Z0-9\u4e00-\u9fa5]/g, "") === normalized,
  );
  if (exact) return 96;
  const partial = field.aliases.some((alias) => {
    const normalizedAlias = alias
      .toUpperCase()
      .replace(/[^A-Z0-9\u4e00-\u9fa5]/g, "");
    // SPEC/DOSE/TYPE 等短字段含义过宽，只允许精确命中，避免误配到监管字段。
    return (
      Math.min(normalized.length, normalizedAlias.length) >= 6 &&
      (normalized.includes(normalizedAlias) ||
        normalizedAlias.includes(normalized))
    );
  });
  if (partial) return 82;
  const key = field.key.toUpperCase();
  if (
    Math.min(normalized.length, key.length) >= 6 &&
    (normalized.includes(key) || key.includes(normalized))
  )
    return 72;
  return 28 + (Math.abs(hash(column + field.key)) % 25);
}

function hash(text) {
  return [...text].reduce(
    (value, character) => ((value << 5) - value + character.charCodeAt(0)) | 0,
    0,
  );
}
