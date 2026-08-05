import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowCounterClockwise,
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
import {
  buildDictionaryValueMappings,
  dictionaryItemValue,
  mergeValueMappingText,
  recommendCostMergeMappings,
} from "./dictionary";

const steps = [
  "连接新系统",
  "选择任务",
  "导入数据源",
  "识别数据",
  "匹配字段",
  "校验修正",
  "执行审计",
];

const initialTargetSystemUrl = "http://10.17.18.88:8000/rbmh-phis/";

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
    hint: "以当前租户 rbmh.base.med.articleType 实时字典为准",
    aliases: ["DRUG_TYPE", "SD_MED", "YPLX", "物品类型"],
    dictionaryId: "rbmh.base.med.articleType",
  },
  {
    key: "sdDose",
    label: "剂型编码",
    required: true,
    group: "基本信息",
    hint: "需映射为新系统剂型字典编码",
    aliases: ["FORM_CODE", "DOSE_FORM", "JXDM", "剂型"],
    dictionaryId: "rbmh.base.med.doseType",
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
    dictionaryId: "rbmh.base.med.usage",
  },
  {
    key: "dftFreq",
    label: "默认频次编码",
    required: false,
    group: "用药规则",
    hint: "草药、耗材可不填",
    aliases: ["FREQ_CODE", "DFT_FREQ", "PCDM", "频次"],
    dictionaryId: "rbmh.base.freq",
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
    dictionaryId: "rbmh.base.med.roundingStrategy",
  },
  {
    key: "sdDps",
    label: "发药方式编码",
    required: false,
    group: "用药规则",
    hint: "需映射为新系统发药方式字典编码",
    aliases: ["DISPENSE_CODE", "SD_DPS", "FYFS", "发药方式"],
    dictionaryId: "rbmh.base.med.dispensingMethod",
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
    required: false,
    group: "包装价格",
    hint: "存在商品信息时必填，例如盒、瓶、支",
    aliases: ["SALE_UNIT", "PACK_UNIT", "UNIT_SALE", "包装单位"],
  },
  {
    key: "unitSaleFactor",
    label: "包装系数",
    required: false,
    group: "包装价格",
    hint: "存在商品信息时必填，必须为正整数",
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
    required: false,
    group: "包装价格",
    hint: "存在商品信息时必填，允许为0，不允许负数",
    aliases: ["BUY_PRICE", "PURCHASE_PRICE", "PRICE_PUR", "进货价"],
  },
  {
    key: "priceSale",
    label: "零售价格",
    required: false,
    group: "包装价格",
    hint: "存在商品信息时必填，允许为0，不允许负数",
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
    dictionaryId: "rbmh.base.med.baseMed",
  },
  {
    key: "fgMedRx",
    label: "处方药标志",
    required: false,
    group: "监管属性",
    hint: "统一转换为0否、1是",
    aliases: ["RX_FLAG", "FG_MED_RX", "CFYP", "处方药品"],
    dictionaryId: "rbmh.base.med.prescriptiondrugIdentification",
  },
  {
    key: "sdChrgitmLv",
    label: "药品档次编码",
    required: false,
    group: "监管属性",
    hint: "原模板“档次”，按新系统字典编码接入",
    aliases: ["CHARGE_LEVEL", "SD_CHRGITM_LV", "YPDC", "档次"],
    dictionaryId: "phis.medicareLevel",
  },
  {
    key: "sdStorage",
    label: "药品储藏编码",
    required: false,
    group: "监管属性",
    hint: "需映射为新系统储藏字典编码",
    aliases: ["STORAGE_CODE", "SD_STORAGE", "YPCC", "药品储藏"],
    dictionaryId: "phis.storageType",
  },
  {
    key: "sdSpeMed",
    label: "特殊药品编码",
    required: false,
    group: "监管属性",
    hint: "需映射为新系统特殊药品字典编码",
    aliases: ["SPECIAL_DRUG_TYPE", "SD_SPE_MED", "TSYP", "特殊药品"],
    dictionaryId: "rbmh.base.med.sdSpeMed",
  },
  {
    key: "sdAllergy",
    label: "过敏类别编码",
    required: false,
    group: "用药规则",
    hint: "来自 HiBdMed 的过敏类别标准字典",
    aliases: ["ALLERGY_CODE", "SD_ALLERGY", "GMLB", "过敏类别"],
    dictionaryId: "rbmh.base.med.sdAllergy",
  },
  {
    key: "fgAntiAppr",
    label: "抗菌药物审批标志",
    required: false,
    group: "抗菌药物",
    hint: "按新系统是否字典转换为0/1",
    aliases: ["ANTI_APPROVAL", "FG_ANTI_APPR", "KJYPSP", "抗菌审批"],
    dictionaryId: "sys.sd.yesOrNo",
  },
  {
    key: "sdProdPlac",
    label: "产地类别编码",
    required: false,
    group: "厂家商品",
    hint: "按新系统国产/进口产地类别字典转换",
    aliases: ["ORIGIN_TYPE", "SD_PROD_PLAC", "CDLB", "产地类别"],
    dictionaryId: "rbmh.base.med.drugGrade",
  },
  {
    key: "fgPois",
    label: "毒麻药品标志",
    required: false,
    group: "监管属性",
    hint: "按新系统是否字典转换为0/1",
    aliases: ["POISON_FLAG", "FG_POIS", "DMBS", "毒麻标志"],
    dictionaryId: "sys.sd.yesOrNo",
  },
  {
    key: "fgAnti",
    label: "抗菌药物标志",
    required: false,
    group: "抗菌药物",
    hint: "按新系统是否字典转换为0/1",
    aliases: ["ANTIBIOTIC_FLAG", "FG_ANTI", "KJYWBS", "抗菌标志"],
    dictionaryId: "sys.sd.yesOrNo",
  },
  {
    key: "fgTcd",
    label: "中药饮片标志",
    required: false,
    group: "监管属性",
    hint: "按新系统是否字典转换为0/1",
    aliases: ["TCM_DECOCTION_FLAG", "FG_TCD", "ZYYPBS", "中药饮片"],
    dictionaryId: "sys.sd.yesOrNo",
  },
  {
    key: "fgSingle",
    label: "允许单开标志",
    required: false,
    group: "用药规则",
    hint: "按新系统是否字典转换为0/1",
    aliases: ["SINGLE_FLAG", "FG_SINGLE", "YXDK", "允许单开"],
    dictionaryId: "sys.sd.yesOrNo",
  },
  {
    key: "fgRegister",
    label: "注册证管理标志",
    required: false,
    group: "监管属性",
    hint: "按新系统注册证管理字典转换",
    aliases: ["REGISTER_FLAG", "FG_REGISTER", "ZCZGL", "需要注册"],
    dictionaryId: "phis.ifNeedRegister",
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
      UNDONE: ["已撤销", "muted"],
      UNDO_PARTIAL: ["部分撤销", "warning"],
    }[status] || [status || "未开始", "muted"]
  );
}

function diffValue(value) {
  if (value === null || value === undefined || `${value}` === "") return "（空）";
  return `${value}`;
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

function Header({ runtime, auth }) {
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
        <span>{auth ? `${auth.userName} · ${auth.roleName}` : "未登录"}</span>
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
  const [targetSystemUrl, setTargetSystemUrl] = useState(
    initialTargetSystemUrl,
  );
  const [targetSystemProbe, setTargetSystemProbe] = useState(null);
  const [loginTenantId, setLoginTenantId] = useState("");
  const [loginPassword, setLoginPassword] = useState("");
  const [targetAuth, setTargetAuth] = useState(null);
  const [dictionaryCatalog, setDictionaryCatalog] = useState(null);
  const [costMergeCatalog, setCostMergeCatalog] = useState(null);
  const [costMergeMappings, setCostMergeMappings] = useState({});
  const [migrationType, setMigrationType] = useState("MEDICINE_BASE");
  const [sourceMode, setSourceMode] = useState("file");
  const [sourceProfile, setSourceProfile] = useState(initialProfile);
  const [legacyInspection, setLegacyInspection] = useState(null);
  const [legacyScope, setLegacyScope] = useState("USED_ACTIVE");
  const [targetProfile, setTargetProfile] = useState(initialProfile);
  const [query, setQuery] = useState("SELECT * FROM T_DRUG_INFO");
  const [sourceName, setSourceName] = useState("尚未选择数据源");
  const [sourceDescription, setSourceDescription] = useState("");
  const [rows, setRows] = useState([]);
  const [columns, setColumns] = useState([]);
  const [sourceKey, setSourceKey] = useState("");
  const [mapping, setMapping] = useState({});
  const [rules, setRules] = useState({});
  const [fieldIndex, setFieldIndex] = useState(0);
  const [expert, setExpert] = useState(false);
  const [allowCreateFactory, setAllowCreateFactory] = useState(false);
  const [conflictStrategy, setConflictStrategy] = useState("INCREMENTAL");
  const [batchDetail, setBatchDetail] = useState(null);
  const [overwritePreview, setOverwritePreview] = useState(null);
  const [selectedOverwriteRowIds, setSelectedOverwriteRowIds] = useState([]);
  const [tenantId, setTenantId] = useState("");
  const [operatorId, setOperatorId] = useState("");
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
  const dictionariesById = useMemo(
    () =>
      Object.fromEntries(
        (dictionaryCatalog?.dictionaries || []).map((dictionary) => [
          dictionary.dicId,
          dictionary,
        ]),
      ),
    [dictionaryCatalog],
  );
  const currentDictionary = currentField?.dictionaryId
    ? dictionariesById[currentField.dictionaryId]
    : null;
  const articleTypeDictionary =
    dictionariesById["rbmh.base.med.articleType"] || null;
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

  async function probeTargetSystem() {
    setBusy("target-system-probe");
    setTargetSystemProbe(null);
    setTargetAuth(null);
    setDictionaryCatalog(null);
    setCostMergeCatalog(null);
    setCostMergeMappings({});
    try {
      const result = await command("probe_target_system", {
        request: { baseUrl: targetSystemUrl },
      });
      setTargetSystemUrl(result.baseUrl);
      setTargetSystemProbe(result);
      notify(`${result.message} · ${result.latencyMs}ms`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function loginTargetSystem(event) {
    event?.preventDefault();
    if (!targetSystemProbe) return fail("请先验证新系统地址");
    setBusy("target-system-login");
    try {
      const result = await command("login_target_system", {
        request: {
          baseUrl: targetSystemProbe.baseUrl,
          tenantId: loginTenantId,
          loginName: "system",
          password: loginPassword,
        },
      });
      const [catalog, costMerges] = await Promise.all([
        command("load_target_dictionaries"),
        command("load_medicine_cost_merges"),
      ]);
      const articleTypes = catalog.dictionaries.find(
        (dictionary) => dictionary.dicId === "rbmh.base.med.articleType",
      );
      setDictionaryCatalog(catalog);
      setCostMergeCatalog(costMerges);
      setCostMergeMappings(
        recommendCostMergeMappings(articleTypes?.items || [], costMerges.items),
      );
      setTargetAuth(result);
      setTenantId(result.tenantId);
      setOperatorId(result.userId);
      setLoginPassword("");
      notify(
        `${result.message}；${catalog.message}；${costMerges.message}${catalog.warnings?.length ? `（${catalog.warnings.length} 个可选字典暂不可用）` : ""}`,
      );
    } catch (error) {
      setTargetAuth(null);
      setDictionaryCatalog(null);
      setCostMergeCatalog(null);
      setCostMergeMappings({});
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function acceptData(data, name, options = {}) {
    if (!data.length) return fail("文件中没有可识别的数据行");
    const detectedColumns = Object.keys(data[0]);
    setRows(data);
    setColumns(detectedColumns);
    setSourceName(name);
    setSourceDescription(options.description || name);
    setSourceKey(
      options.sourceKey && detectedColumns.includes(options.sourceKey)
        ? options.sourceKey
        : detectedColumns[0] || "",
    );
    const auto = {};
    targetFields.forEach((field) => {
      const best = detectedColumns
        .map((column) => ({ column, score: matchScore(column, field) }))
        .sort((a, b) => b.score - a.score)[0];
      if (best?.score >= 55) auto[field.key] = best.column;
    });
    setMapping(auto);
    setStep(3);
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
      if (preview.truncated)
        return fail("自定义查询结果超过500行，请收窄范围或使用 PHIS27 自动模板");
      acceptData(
        preview.rows,
        `${databaseKinds[sourceProfile.kind]?.label || sourceProfile.kind} · ${sourceProfile.database} · 自定义只读查询`,
        { description: query },
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

  async function inspectPhis27Source() {
    setBusy("phis27-inspect");
    setLegacyInspection(null);
    try {
      const checked = await command("test_database_connection", {
        profile: sourceProfile,
      });
      const inspection = await command("inspect_phis27_source", {
        profile: sourceProfile,
      });
      setLegacyInspection(inspection);
      const recommended = inspection.scopes?.find((scope) => scope.recommended);
      if (recommended) setLegacyScope(recommended.id);
      if (!inspection.detected) return fail(inspection.message);
      notify(`${checked.message}；${inspection.message}`);
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function loadPhis27Medicine() {
    if (!legacyInspection?.detected) return fail("请先识别 PHIS27 数据结构");
    const selected = legacyInspection.scopes.find(
      (scope) => scope.id === legacyScope,
    );
    setBusy("phis27-load");
    try {
      const preview = await command("load_phis27_medicine", {
        request: {
          connection: sourceProfile,
          scope: legacyScope,
          limit: 10000,
        },
      });
      if (preview.truncated)
        return fail("当前范围超过单批10,000行，请缩小范围后再读取");
      setAllowCreateFactory(true);
      acceptData(
        preview.rows,
        `PHIS27 · ${legacyInspection.schema} · ${selected?.label || legacyScope}`,
        {
          sourceKey: "SOURCE_KEY",
          description: `PHIS27内置模板:${legacyScope}; schema=${legacyInspection.schema}`,
        },
      );
      notify(
        `已按安全模板读取 ${preview.rows.length} 行，来源主键使用 YPXH:YPCD`,
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  function beginMapping() {
    setFieldIndex(0);
    setStep(4);
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

  function dictionaryForField(field) {
    return field?.dictionaryId ? dictionariesById[field.dictionaryId] : null;
  }

  function autoMapDictionary(field) {
    const dictionary = dictionaryForField(field);
    const sourceField = mapping[field.key];
    if (!dictionary || !sourceField) {
      return fail("请先为该目标字段选择老系统来源字段");
    }
    const additions = buildDictionaryValueMappings(
      rows.map((row) => row[sourceField]),
      dictionary.items,
    );
    const count = Object.keys(additions).length;
    if (!count) {
      return fail("没有找到可安全自动匹配的字典名称或编码，请人工确认");
    }
    setRules((current) => ({
      ...current,
      [field.key]: {
        ...current[field.key],
        valueMappingsText: mergeValueMappingText(
          current[field.key]?.valueMappingsText,
          additions,
        ),
      },
    }));
    notify(`已为“${field.label}”生成 ${count} 条目标字典映射`);
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
          sourceDescription: sourceDescription || query,
          conflictStrategy,
          allowCreateFactory,
          idempotencyKey: objectId(),
          mappings,
          costMergeMappings,
          rows: preparedRows,
        },
      });
      setBatchDetail(detail);
      setOverwritePreview(null);
      setSelectedOverwriteRowIds([]);
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

  async function previewOverwrite() {
    if (!batchDetail) return;
    setBusy("overwrite-preview");
    try {
      const preview = await command("preview_overwrite_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setOverwritePreview(preview);
      setSelectedOverwriteRowIds(
        preview.rows
          .filter((row) => row.action !== "UNCHANGED")
          .map((row) => row.rowId),
      );
      notify(preview.message);
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
    const overwrite = batchDetail.batch.conflictStrategy === "OVERWRITE";
    if (overwrite && !overwritePreview)
      return fail("覆盖迁移必须先生成并确认目标字段差异");
    if (overwrite && !selectedOverwriteRowIds.length)
      return fail("请至少勾选一条需要执行的记录");
    setBusy(failedOnly ? "retry" : "execute");
    try {
      const detail = await command("execute_migration_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
          failedOnly,
          tenantId,
          operatorId,
          organizationId: "",
          selectedRowIds: overwrite ? selectedOverwriteRowIds : [],
          overwritePreviewConfirmed: overwrite && Boolean(overwritePreview),
        },
      });
      setBatchDetail(detail);
      setStep(6);
      notify(failedOnly ? "失败行重试完成" : "迁移执行完成");
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  async function undoBatch() {
    if (!batchDetail) return;
    const confirmed = window.confirm(
      "仅会删除本批次由工具新增、且尚未被后续数据引用的记录；复用数据不会删除。确认撤销？",
    );
    if (!confirmed) return;
    setBusy("undo");
    try {
      const detail = await command("undo_migration_batch", {
        request: {
          batchId: batchDetail.batch.batchId,
          target: targetProfile,
        },
      });
      setBatchDetail(detail);
      setActiveResultTab("audit");
      notify(
        detail.batch.status === "UNDONE"
          ? "该批次新增数据已安全撤销"
          : "撤销已完成；有后续引用的数据已保留，请查看审计日志",
      );
    } catch (error) {
      fail(error);
    } finally {
      setBusy("");
    }
  }

  return (
    <div className="app-shell">
      <Header runtime={runtime} auth={targetAuth} />
      <Stepper active={step} />
      <main className="workspace">
        {step === 0 && (
          <section className="screen access-screen">
            <div className="screen-heading">
              <span className="eyebrow">第 1 步 · 新系统身份确认</span>
              <h1>先连接要迁入的新系统</h1>
              <p>
                地址可用、取得 system 租户管理员角色并成功建立 tk 登录会话后，才开放数据迁移功能。密码和 tk 仅在本次运行内存中使用。
              </p>
            </div>

            <div className="access-grid">
              <div className="access-card">
                <div className="access-card__number">1</div>
                <div className="access-card__body">
                  <div className="section-title">
                    <Plugs size={21} weight="duotone" />
                    <div>
                      <strong>验证新系统地址</strong>
                      <small>检查服务是否可以从当前电脑访问</small>
                    </div>
                  </div>
                  <div className="access-inline-form">
                    <Field label="新系统访问地址" wide>
                      <input
                        value={targetSystemUrl}
                        onChange={(event) => {
                          setTargetSystemUrl(event.target.value);
                          setTargetSystemProbe(null);
                          setTargetAuth(null);
                          setDictionaryCatalog(null);
                          setCostMergeCatalog(null);
                          setCostMergeMappings({});
                        }}
                        placeholder="http://服务器:端口/rbmh-phis"
                        spellCheck="false"
                      />
                    </Field>
                    <button
                      className="button button--secondary"
                      disabled={busy === "target-system-probe"}
                      onClick={probeTargetSystem}
                    >
                      <Plugs size={19} />
                      {busy === "target-system-probe"
                        ? "正在验证…"
                        : "验证地址"}
                    </button>
                  </div>
                  {targetSystemProbe && (
                    <div className="access-result access-result--success">
                      <CheckCircle size={19} weight="fill" />
                      <div>
                        <strong>地址可用</strong>
                        <span>
                          HTTP {targetSystemProbe.statusCode} · {targetSystemProbe.latencyMs}ms
                        </span>
                      </div>
                    </div>
                  )}
                  {targetSystemUrl.trim().toLowerCase().startsWith("http://") && (
                    <div className="access-result access-result--warning">
                      <Warning size={19} weight="fill" />
                      <div>
                        <strong>当前使用内网 HTTP</strong>
                        <span>MD5 仅符合登录接口协议，不等同于传输加密；正式环境建议启用 HTTPS。</span>
                      </div>
                    </div>
                  )}
                </div>
              </div>

              <form
                className={`access-card ${!targetSystemProbe ? "access-card--locked" : ""}`}
                onSubmit={loginTargetSystem}
              >
                <div className="access-card__number">2</div>
                <div className="access-card__body">
                  <div className="section-title">
                    <LockKey size={21} weight="duotone" />
                    <div>
                      <strong>租户管理员认证与角色登录</strong>
                      <small>依次调用 myRoles、myApps，仅允许 system 账号</small>
                    </div>
                  </div>
                  <div className="auth-form-grid">
                    <Field label="租户编码">
                      <input
                        value={loginTenantId}
                        disabled={!targetSystemProbe || Boolean(targetAuth)}
                        onChange={(event) => setLoginTenantId(event.target.value)}
                        placeholder="例如 cszzyzh"
                        autoComplete="organization"
                      />
                    </Field>
                    <Field label="登录账号">
                      <input value="system" disabled aria-label="登录账号" />
                    </Field>
                    <Field label="system 密码" wide>
                      <div className="password-input">
                        <LockKey size={17} />
                        <input
                          type="password"
                          value={loginPassword}
                          disabled={!targetSystemProbe || Boolean(targetAuth)}
                          onChange={(event) => setLoginPassword(event.target.value)}
                          placeholder="请输入 system 密码"
                          autoComplete="current-password"
                        />
                      </div>
                    </Field>
                  </div>
                  {!targetAuth ? (
                    <button
                      className="button button--primary button--wide"
                      type="submit"
                      disabled={
                        !targetSystemProbe || busy === "target-system-login"
                      }
                    >
                      <ShieldCheck size={19} />
                      {busy === "target-system-login"
                        ? "正在认证…"
                        : "认证并建立 tk 登录会话"}
                    </button>
                  ) : (
                    <div className="access-result access-result--success auth-result">
                      <ShieldCheck size={20} weight="fill" />
                      <div>
                        <strong>
                          {targetAuth.tenantName || targetAuth.tenantId}
                        </strong>
                        <span>
                          {targetAuth.userName} · {targetAuth.roleName} · {targetAuth.roleCd}
                        </span>
                        {dictionaryCatalog && (
                          <span>
                            已同步 {dictionaryCatalog.dictionaries.length} 个药品标准字典
                            {dictionaryCatalog.warnings?.length
                              ? ` · ${dictionaryCatalog.warnings.length} 个可选字典待处理`
                              : ""}
                          </span>
                        )}
                        {costMergeCatalog && (
                          <span>已同步 {costMergeCatalog.items.length} 个费用归并项目</span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              </form>
            </div>

            <div className="security-banner">
              <ShieldCheck size={20} weight="fill" />
              <div>
                <strong>双重门禁</strong>
                <span>
                  前端固定 system，桌面后端会再次检查账号、租户、角色编码、启用状态和 tk 下发结果，后续服务请求自动携带 Cookie。
                </span>
              </div>
            </div>
            <div className="screen-actions screen-actions--end">
              <button
                className="button button--primary"
                disabled={!targetAuth}
                onClick={() => setStep(1)}
              >
                认证完成，选择迁移任务
                <ArrowRight />
              </button>
            </div>
          </section>
        )}

        {step === 1 && (
          <section className="screen task-screen">
            <div className="screen-heading screen-heading--row">
              <div>
                <span className="eyebrow">第 2 步 · 迁移任务</span>
                <h1>这次要迁移什么？</h1>
                <p>两类任务相互独立，库存初始化必须建立在药品基础信息已经同步的前提下。</p>
              </div>
              <div className="source-summary">
                <ShieldCheck size={24} />
                <div>
                  <small>已认证新系统</small>
                  <strong>
                    {targetAuth?.tenantName || targetAuth?.tenantId} · system
                  </strong>
                </div>
              </div>
            </div>
            <div className="task-grid">
              <button
                className={`task-card ${migrationType === "MEDICINE_BASE" ? "task-card--selected" : ""}`}
                onClick={() => setMigrationType("MEDICINE_BASE")}
              >
                <span className="task-card__icon">
                  <FirstAidKit size={28} weight="duotone" />
                </span>
                <span className="task-card__copy">
                  <small>任务 A · 可用</small>
                  <strong>药品基础信息同步</strong>
                  <p>
                    同步通用药品、单位、别名、生产厂家和药品商品信息，为后续机构库存初始化建立主键对照。
                  </p>
                  <em>全局数据 · 支持预校验、幂等复用和失败重试</em>
                </span>
                <span className="task-card__check">
                  {migrationType === "MEDICINE_BASE" && (
                    <Check size={17} weight="bold" />
                  )}
                </span>
              </button>
              <button className="task-card task-card--disabled" disabled>
                <span className="task-card__icon">
                  <Database size={28} weight="duotone" />
                </span>
                <span className="task-card__copy">
                  <small>任务 B · 下一轮接入</small>
                  <strong>机构库房初始化</strong>
                  <p>
                    选择具体机构、药库或药房，在基础药品匹配完整后按批次同步库存数量、价格、批号和效期。
                  </p>
                  <em>机构数据 · 需要库房映射和库存初始化防重锁</em>
                </span>
                <span className="task-card__badge">前置能力建设中</span>
              </button>
            </div>
            <div className="prerequisite-note">
              <Info size={19} />
              <span>
                当前先完成药品基础信息同步闭环。机构库房初始化将在目标库存表结构和机构、库房接口明确后开放。
              </span>
            </div>
            <div className="screen-actions">
              <button
                className="button button--secondary"
                onClick={() => setStep(0)}
              >
                <ArrowLeft />
                返回认证
              </button>
              <button
                className="button button--primary"
                onClick={() => setStep(2)}
              >
                进入药品基础信息同步
                <ArrowRight />
              </button>
            </div>
          </section>
        )}

        {step === 2 && (
          <section className="screen source-screen">
            <div className="screen-heading">
              <span className="eyebrow">第 3 步 · 数据来源</span>
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
                  onChange={(next) => {
                    setSourceProfile(next);
                    setLegacyInspection(null);
                  }}
                  title="老系统只读连接"
                  drivers={databaseDrivers}
                  driverPacks={driverPacks}
                />
                {sourceProfile.kind === "oracle" && (
                  <div className="legacy-adapter">
                    <div className="legacy-adapter__heading">
                      <div>
                        <span className="eyebrow">内置适配器</span>
                        <strong>自动识别 Bsoft PHIS27 药品数据</strong>
                        <small>
                          自动核对核心表、统计数据范围，并生成只读多表组合查询。
                        </small>
                      </div>
                      <button
                        className="button button--secondary"
                        disabled={busy === "phis27-inspect"}
                        onClick={inspectPhis27Source}
                      >
                        <ListMagnifyingGlass size={19} />
                        {busy === "phis27-inspect"
                          ? "正在识别…"
                          : "识别 PHIS27"}
                      </button>
                    </div>
                    {legacyInspection && (
                      <div className="legacy-inspection">
                        <div
                          className={`access-result ${legacyInspection.detected ? "access-result--success" : "access-result--warning"}`}
                        >
                          {legacyInspection.detected ? (
                            <CheckCircle size={19} weight="fill" />
                          ) : (
                            <Warning size={19} weight="fill" />
                          )}
                          <div>
                            <strong>{legacyInspection.message}</strong>
                            <span>
                              Schema {legacyInspection.schema} · 已核对 {legacyInspection.checkedTables.length} 张表
                            </span>
                          </div>
                        </div>
                        {legacyInspection.detected && (
                          <>
                            <div className="legacy-stats">
                              {[
                                ["通用药品", legacyInspection.totalMedicines],
                                ["机构配置", legacyInspection.configuredMedicines],
                                ["在用药品", legacyInspection.activeConfiguredMedicines],
                                ["厂家商品", legacyInspection.productRows],
                                ["有库存药品", legacyInspection.stockMedicines],
                              ].map(([label, value]) => (
                                <div key={label}>
                                  <span>{label}</span>
                                  <strong>{value}</strong>
                                </div>
                              ))}
                            </div>
                            <div className="legacy-scopes">
                              {legacyInspection.scopes.map((scope) => (
                                <label
                                  className={`legacy-scope ${legacyScope === scope.id ? "legacy-scope--selected" : ""}`}
                                  key={scope.id}
                                >
                                  <input
                                    type="radio"
                                    name="legacy-scope"
                                    checked={legacyScope === scope.id}
                                    onChange={() => setLegacyScope(scope.id)}
                                  />
                                  <span>
                                    <strong>
                                      {scope.label}
                                      {scope.recommended && <em>推荐</em>}
                                    </strong>
                                    <small>{scope.description}</small>
                                  </span>
                                  <b>
                                    {scope.medicineCount} 种 / 约 {scope.estimatedRows} 行
                                  </b>
                                </label>
                              ))}
                            </div>
                            <div className="legacy-warnings">
                              {legacyInspection.warnings.map((warning) => (
                                <span key={warning}>
                                  <Warning size={15} />
                                  {warning}
                                </span>
                              ))}
                            </div>
                            <button
                              className="button button--primary button--wide"
                              disabled={busy === "phis27-load"}
                              onClick={loadPhis27Medicine}
                            >
                              <Rows size={19} />
                              {busy === "phis27-load"
                                ? "正在读取标准数据…"
                                : "按选定范围读取标准药品数据"}
                            </button>
                          </>
                        )}
                      </div>
                    )}
                  </div>
                )}
                <div className="custom-query-divider">
                  <span>或使用自定义只读 SQL</span>
                </div>
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

        {step === 3 && (
          <section className="screen">
            <div className="screen-heading screen-heading--row">
              <div>
                <span className="eyebrow">第 4 步 · 识别数据</span>
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
                onClick={() => setStep(2)}
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

        {step === 4 && (
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
            {currentDictionary && (
              <div className="dictionary-guide">
                <div>
                  <strong>新系统标准字典</strong>
                  <code>{currentDictionary.dicId}</code>
                  <span>{currentDictionary.items.length} 个可用值</span>
                </div>
                <div className="dictionary-guide__items">
                  {currentDictionary.items.slice(0, 12).map((item) => (
                    <span key={item.id || dictionaryItemValue(item)}>
                      <b>{dictionaryItemValue(item)}</b>
                      {item.text || item.na}
                    </span>
                  ))}
                  {currentDictionary.items.length > 12 && (
                    <em>另有 {currentDictionary.items.length - 12} 项</em>
                  )}
                </div>
                <button
                  className="button button--secondary"
                  type="button"
                  disabled={!mapping[currentField.key]}
                  onClick={() => autoMapDictionary(currentField)}
                >
                  <LinkSimple size={17} />
                  按名称/编码自动生成转换
                </button>
              </div>
            )}
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
                  fieldIndex ? setFieldIndex(fieldIndex - 1) : setStep(3)
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
                    else setStep(5);
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

        {step === 5 && (
          <section className="screen">
            <div className="screen-heading">
              <span className="eyebrow">第 6 步 · 校验与修正</span>
              <h1>先试迁移，再决定是否正式写入</h1>
              <p>
                此阶段只把原始数据、转换结果和问题写入本地暂存库，不会修改新系统业务表。
              </p>
            </div>
            <div className="rules-layout">
              <div className="rules-card cost-merge-card">
                <div className="section-title">
                  <LinkSimple size={21} />
                  <div>
                    <strong>药品类型 → 费用归并</strong>
                    <small>
                      费用归并不是标准字典；系统已按药品类型生成推荐值，执行前可逐项确认
                    </small>
                  </div>
                </div>
                <div className="cost-merge-grid">
                  {(articleTypeDictionary?.items || []).map((article) => {
                    const articleKey = dictionaryItemValue(article);
                    return (
                      <label key={articleKey}>
                        <span>
                          <b>{articleKey}</b>
                          {article.text || article.na}
                        </span>
                        <select
                          value={costMergeMappings[articleKey] || ""}
                          onChange={(event) =>
                            setCostMergeMappings((current) => ({
                              ...current,
                              [articleKey]: event.target.value,
                            }))
                          }
                        >
                          <option value="">未设置，相关药品将校验失败</option>
                          {(costMergeCatalog?.items || []).map((cost) => (
                            <option value={cost.key} key={cost.key}>
                              {cost.text} · {cost.key}
                            </option>
                          ))}
                        </select>
                      </label>
                    );
                  })}
                </div>
              </div>
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
                      增量模式会跳过来源未变化的数据；检测到来源已变化时阻止静默覆盖。
                      新数据仍按药品及商品业务键判重
                    </small>
                  </span>
                  <select
                    value={conflictStrategy}
                    onChange={(event) =>
                      setConflictStrategy(event.target.value)
                    }
                  >
                    <option value="INCREMENTAL">新增增量迁移（推荐）</option>
                    <option value="FAIL">发现重复即报错</option>
                    <option value="OVERWRITE">覆盖迁移（保存原值，可撤销）</option>
                  </select>
                </label>
                {conflictStrategy === "OVERWRITE" && (
                  <div className="strategy-warning">
                    仅覆盖本工具台账确认管理的药品和商品；共享厂家、包装单位不会直接改写。
                    修改前字段值会进入审计快照，撤销时自动恢复。
                  </div>
                )}
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
                        {dictionaryForField(field) ? (
                          <select
                            aria-label={`${field.label}缺失时默认值`}
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
                          >
                            <option value="">缺失时不设置默认值</option>
                            {dictionaryForField(field).items.map((item) => (
                              <option
                                value={dictionaryItemValue(item)}
                                key={item.id || dictionaryItemValue(item)}
                              >
                                {dictionaryItemValue(item)} · {item.text || item.na}
                              </option>
                            ))}
                          </select>
                        ) : (
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
                        )}
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
                            {dictionaryForField(field) && (
                              <div className="dictionary-editor-head">
                                <span>
                                  目标字典：{field.dictionaryId} ·{" "}
                                  {dictionaryForField(field).items.length} 项
                                </span>
                                <button
                                  type="button"
                                  className="button button--ghost"
                                  onClick={() => autoMapDictionary(field)}
                                >
                                  自动匹配当前来源值
                                </button>
                              </div>
                            )}
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
                onClick={() => setStep(4)}
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
                  onClick={batchDetail ? () => setStep(6) : prepareBatch}
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

        {step === 6 && (
          <section className="screen">
            <div className="screen-heading screen-heading--row">
              <div>
                <span className="eyebrow">第 7 步 · 执行与审计</span>
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
              !["SUCCESS", "PARTIAL", "UNDONE", "UNDO_PARTIAL"].includes(
                batchDetail.batch.status,
              ) && (
                <div className="target-card">
                  <ConnectionForm
                    value={targetProfile}
                    onChange={(profile) => {
                      setTargetProfile(profile);
                      setOverwritePreview(null);
                      setSelectedOverwriteRowIds([]);
                    }}
                    title="新系统目标数据库"
                    drivers={databaseDrivers}
                    driverPacks={driverPacks}
                  />
                  <div className="target-context">
                    <Field label="租户 ID">
                      <input
                        value={tenantId}
                        disabled
                        title="来自已认证的新系统租户"
                      />
                    </Field>
                    <Field label="操作人 ID">
                      <input
                        value={operatorId}
                        disabled
                        title="来自已认证的 system 用户"
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
                    {batchDetail.batch.conflictStrategy === "OVERWRITE" && (
                      <button
                        className="button button--secondary"
                        disabled={busy === "overwrite-preview"}
                        onClick={previewOverwrite}
                      >
                        <ListMagnifyingGlass />
                        {busy === "overwrite-preview"
                          ? "正在读取差异…"
                          : overwritePreview
                            ? "重新读取覆盖差异"
                            : "读取并确认覆盖差异"}
                      </button>
                    )}
                    <button
                      className="button button--danger"
                      disabled={
                        busy === "execute" ||
                        !batchDetail.batch.validCount ||
                        (batchDetail.batch.conflictStrategy === "OVERWRITE" &&
                          (!overwritePreview ||
                            !selectedOverwriteRowIds.length))
                      }
                      onClick={() => execute(false)}
                    >
                      <Play weight="fill" />
                      {busy === "execute"
                        ? "正在逐行写入…"
                        : batchDetail.batch.conflictStrategy === "OVERWRITE"
                          ? `执行已确认的 ${selectedOverwriteRowIds.length} 行`
                          : `正式迁移 ${batchDetail.batch.validCount} 行`}
                    </button>
                  </div>
                </div>
              )}
            {overwritePreview &&
              batchDetail?.batch.conflictStrategy === "OVERWRITE" &&
              !["SUCCESS", "UNDONE", "UNDO_PARTIAL"].includes(
                batchDetail.batch.status,
              ) && (
                <div className="overwrite-preview-card">
                  <div className="overwrite-preview-heading">
                    <div>
                      <span className="eyebrow">覆盖前确认</span>
                      <h2>逐条核对目标库字段变化</h2>
                      <p>{overwritePreview.message}</p>
                    </div>
                    <label>
                      <input
                        type="checkbox"
                        checked={
                          selectedOverwriteRowIds.length > 0 &&
                          selectedOverwriteRowIds.length ===
                            overwritePreview.rows.filter(
                              (row) => row.action !== "UNCHANGED",
                            ).length
                        }
                        onChange={(event) =>
                          setSelectedOverwriteRowIds(
                            event.target.checked
                              ? overwritePreview.rows
                                  .filter(
                                    (row) => row.action !== "UNCHANGED",
                                  )
                                  .map((row) => row.rowId)
                              : [],
                          )
                        }
                      />
                      全选有变化记录
                    </label>
                  </div>
                  <div className="overwrite-preview-list">
                    {overwritePreview.rows.map((row) => {
                      const selected = selectedOverwriteRowIds.includes(
                        row.rowId,
                      );
                      return (
                        <article
                          key={row.rowId}
                          className={selected ? "selected" : ""}
                        >
                          <header>
                            <label>
                              <input
                                type="checkbox"
                                disabled={row.action === "UNCHANGED"}
                                checked={selected}
                                onChange={(event) =>
                                  setSelectedOverwriteRowIds((current) =>
                                    event.target.checked
                                      ? [...new Set([...current, row.rowId])]
                                      : current.filter((id) => id !== row.rowId),
                                  )
                                }
                              />
                              <strong>
                                #{row.rowNo} · {row.sourceKey}
                              </strong>
                            </label>
                            <span
                              className={`diff-action diff-action--${row.action.toLowerCase()}`}
                            >
                              {row.action === "INSERT"
                                ? "新增"
                                : row.action === "UPDATE"
                                  ? `覆盖 ${row.changes.length} 项`
                                  : "无变化"}
                            </span>
                          </header>
                          <p>{row.message}</p>
                          {row.changes.length > 0 && (
                            <div className="field-diff-list">
                              {row.changes.map((change) => (
                                <div
                                  key={`${change.table}.${change.column}`}
                                >
                                  <strong>{change.label}</strong>
                                  <code>{diffValue(change.before)}</code>
                                  <ArrowRight />
                                  <code>{diffValue(change.after)}</code>
                                </div>
                              ))}
                            </div>
                          )}
                        </article>
                      );
                    })}
                  </div>
                </div>
              )}
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
                      disabled={
                        busy === "retry" ||
                        ["UNDONE", "UNDO_PARTIAL"].includes(
                          batchDetail.batch.status,
                        )
                      }
                      onClick={() => execute(true)}
                    >
                      {busy === "retry" ? "重试中…" : "仅重试失败行"}
                    </button>
                  )}
                  {["SUCCESS", "PARTIAL"].includes(
                    batchDetail.batch.status,
                  ) && (
                    <button
                      className="undo-button"
                      disabled={busy === "undo"}
                      onClick={undoBatch}
                    >
                      <ArrowCounterClockwise />
                      {busy === "undo" ? "撤销中…" : "撤销本批次新增数据"}
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
                  setStep(5);
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
