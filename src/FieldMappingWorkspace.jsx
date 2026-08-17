import { useDeferredValue, useEffect, useMemo, useState } from "react";
import {
  ArrowRight,
  ArrowCounterClockwise,
  CaretLeft,
  CaretRight,
  CheckCircle,
  FloppyDisk,
  Info,
  LinkSimple,
  MagnifyingGlass,
  PencilSimple,
  Plus,
  Trash,
  Warning,
  X,
} from "@phosphor-icons/react";
import { dictionaryItemValue } from "./dictionary";
import { firstPresentValue } from "./migrationPreview";
import { SearchableSelect } from "./SearchableSelect";
import { IGNORE_VALUE_MAPPING_TARGET } from "./transforms";

const mappingFilters = [
  { key: "ALL", label: "全部" },
  { key: "PENDING", label: "待处理" },
  { key: "DICTIONARY", label: "字典待确认" },
  { key: "READY", label: "已完成" },
];

const dictionaryFilters = [
  { key: "ALL", label: "全部" },
  { key: "UNMATCHED", label: "未匹配" },
  { key: "MATCHED", label: "已匹配" },
  { key: "IGNORED", label: "已忽略" },
];

function dictionaryRowState(item) {
  if (item.ignored) return "IGNORED";
  if (item.matched) return "MATCHED";
  return "UNMATCHED";
}

function filterStatus(status, filter) {
  if (filter === "PENDING") {
    return ["pending", "optional"].includes(status.state);
  }
  if (filter === "DICTIONARY") return status.state === "dictionary";
  if (filter === "READY") return status.state === "ready";
  return true;
}

function toMedicineListItem(entry) {
  const row = entry.row || {};
  return {
    rowIndex: entry.rowIndex,
    sourceKey: firstPresentValue(row, [
      "SOURCE_KEY",
      "SOURCE_MED_PRO_KEY",
      "DRUG_CODE",
      "_sourceKey",
    ]),
    name: firstPresentValue(row, [
      "DRUG_NAME",
      "YPMC",
      "GENERIC_NAME",
      "naMed",
    ]),
    specification: firstPresentValue(row, [
      "SPEC",
      "YPGG",
      "DRUG_SPEC",
      "spec",
    ]),
    dosageForm: firstPresentValue(row, [
      "FORM_CODE",
      "YPSX",
      "DOSE_FORM",
      "sdDose",
    ]),
    unit: firstPresentValue(row, [
      "PRE_UNIT",
      "ZXDW",
      "YPDW",
      "MIN_UNIT",
      "unitPre",
    ]),
    manufacturer: firstPresentValue(row, [
      "FACTORY_NAME",
      "CDQC",
      "CDMC",
      "MANUFACTURER",
      "naFac",
    ]),
    productName: firstPresentValue(row, [
      "PRODUCT_NAME",
      "YBSPMC",
      "BRAND_NAME",
      "naMedPro",
    ]),
  };
}

function SourceValueMedicineDialog({ field, item, onClose }) {
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(0);
  const deferredQuery = useDeferredValue(
    query.trim().toLocaleLowerCase("zh-CN"),
  );
  const medicines = useMemo(
    () => (item.medicines || []).map(toMedicineListItem),
    [item.medicines],
  );
  const filtered = useMemo(() => {
    if (!deferredQuery) return medicines;
    return medicines.filter((medicine) =>
      Object.values(medicine)
        .join(" ")
        .toLocaleLowerCase("zh-CN")
        .includes(deferredQuery),
    );
  }, [deferredQuery, medicines]);
  const pageSize = 40;
  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const safePage = Math.min(page, pageCount - 1);
  const visible = filtered.slice(
    safePage * pageSize,
    (safePage + 1) * pageSize,
  );

  useEffect(() => {
    const closeOnEscape = (event) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose]);

  const sourceLabel = item.sourceIsBlank
    ? "空值"
    : item.sourceText || `来源编码 ${item.sourceValue}`;

  return (
    <div
      className="dictionary-medicine-backdrop"
      onMouseDown={onClose}
      role="presentation"
    >
      <section
        aria-label={`${sourceLabel}对应药品列表`}
        aria-modal="true"
        className="dictionary-medicine-dialog"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header className="dictionary-medicine-dialog__header">
          <div>
            <span className="eyebrow">来源值药品明细</span>
            <h2>{sourceLabel}</h2>
            <p>
              {field.label} ·{" "}
              {item.sourceIsBlank ? "来源为空" : `来源编码 ${item.sourceValue}`}{" "}
              · 共 {item.count} 条药品
            </p>
          </div>
          <button
            aria-label="关闭药品列表"
            className="icon-button"
            onClick={onClose}
            type="button"
          >
            <X size={19} />
          </button>
        </header>
        <div className="dictionary-medicine-dialog__toolbar">
          <label>
            <MagnifyingGlass size={16} />
            <input
              aria-label="搜索对应药品"
              onChange={(event) => {
                setQuery(event.target.value);
                setPage(0);
              }}
              placeholder="搜索药品名称、规格、厂家、商品名或来源键"
              type="search"
              value={query}
            />
          </label>
          <span>
            当前显示 <b>{filtered.length}</b> / {medicines.length} 条
          </span>
        </div>
        <div className="dictionary-medicine-table-wrap">
          <table className="dictionary-medicine-table">
            <thead>
              <tr>
                <th>来源</th>
                <th>药品名称</th>
                <th>规格 / 剂型</th>
                <th>单位</th>
                <th>厂家 / 商品名</th>
              </tr>
            </thead>
            <tbody>
              {visible.map((medicine) => (
                <tr key={`${medicine.rowIndex}-${medicine.sourceKey}`}>
                  <td>
                    <strong>第 {medicine.rowIndex + 1} 行</strong>
                    <small>{medicine.sourceKey || "—"}</small>
                  </td>
                  <td>{medicine.name || "未命名药品"}</td>
                  <td>
                    <strong>{medicine.specification || "—"}</strong>
                    <small>{medicine.dosageForm || "剂型未提供"}</small>
                  </td>
                  <td>{medicine.unit || "—"}</td>
                  <td>
                    <strong>{medicine.manufacturer || "—"}</strong>
                    <small>{medicine.productName || "商品名未提供"}</small>
                  </td>
                </tr>
              ))}
              {!visible.length && (
                <tr>
                  <td className="dictionary-medicine-table__empty" colSpan="5">
                    没有符合搜索条件的药品
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <footer className="dictionary-medicine-dialog__footer">
          <span>
            第 {safePage + 1} / {pageCount} 页 · 每页最多 {pageSize} 条
          </span>
          <div>
            <button
              className="button button--secondary"
              disabled={safePage === 0}
              onClick={() => setPage((current) => Math.max(0, current - 1))}
              type="button"
            >
              <CaretLeft size={15} /> 上一页
            </button>
            <button
              className="button button--secondary"
              disabled={safePage >= pageCount - 1}
              onClick={() =>
                setPage((current) => Math.min(pageCount - 1, current + 1))
              }
              type="button"
            >
              下一页 <CaretRight size={15} />
            </button>
          </div>
        </footer>
      </section>
    </div>
  );
}

export function FieldMappingNavigator({
  currentFieldKey,
  onSelectField,
  statuses,
  targetLocations,
}) {
  const [filter, setFilter] = useState("ALL");
  const [query, setQuery] = useState("");
  const normalizedQuery = query.trim().toLocaleLowerCase("zh-CN");
  const filteredStatuses = useMemo(
    () =>
      statuses.filter((status) => {
        if (!filterStatus(status, filter)) return false;
        if (!normalizedQuery) return true;
        const field = status.field;
        return `${field.label} ${field.key} ${field.group} ${targetLocations(field).join(" ")} ${status.sourceField}`
          .toLocaleLowerCase("zh-CN")
          .includes(normalizedQuery);
      }),
    [filter, normalizedQuery, statuses, targetLocations],
  );
  const grouped = filteredStatuses.reduce((result, status) => {
    const group = status.field.group || "其他";
    if (!result.has(group)) result.set(group, []);
    result.get(group).push(status);
    return result;
  }, new Map());
  const readyCount = statuses.filter((item) => item.state === "ready").length;
  const dictionaryCount = statuses.filter(
    (item) => item.state === "dictionary",
  ).length;
  const pendingCount = statuses.length - readyCount - dictionaryCount;

  return (
    <aside className="mapping-navigator" aria-label="字段配置导航">
      <div className="mapping-navigator__heading">
        <div>
          <strong>字段导航</strong>
          <span>点击任意字段直接配置</span>
        </div>
        <b title="来源字段及字典值均已完成">
          就绪 {readyCount}/{statuses.length}
        </b>
      </div>
      <div className="mapping-navigator__summary">
        <span>
          <b>{pendingCount}</b> 待处理
        </span>
        <span className={dictionaryCount ? "is-warning" : ""}>
          <b>{dictionaryCount}</b> 字典待确认
        </span>
      </div>
      <label className="mapping-navigator__search">
        <MagnifyingGlass size={15} />
        <input
          aria-label="搜索目标字段"
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索名称、字段或来源"
          type="search"
          value={query}
        />
      </label>
      <div className="mapping-navigator__filters" aria-label="字段状态筛选">
        {mappingFilters.map((item) => (
          <button
            className={filter === item.key ? "is-active" : ""}
            key={item.key}
            onClick={() => setFilter(item.key)}
            type="button"
          >
            {item.label}
          </button>
        ))}
      </div>
      <div className="mapping-navigator__list">
        {[...grouped.entries()].map(([group, groupStatuses]) => (
          <section key={group}>
            <div className="mapping-navigator__group">
              <strong>{group}</strong>
              <span>{groupStatuses.length}</span>
            </div>
            {groupStatuses.map((status) => (
              <button
                aria-current={
                  currentFieldKey === status.field.key ? "step" : undefined
                }
                className={`mapping-nav-item mapping-nav-item--${status.state} ${currentFieldKey === status.field.key ? "is-current" : ""}`}
                key={status.field.key}
                onClick={() => onSelectField(status.index)}
                type="button"
              >
                <span className="mapping-nav-item__state">
                  {status.state === "ready" ? (
                    <CheckCircle size={16} weight="fill" />
                  ) : status.state === "dictionary" ? (
                    <Warning size={16} weight="fill" />
                  ) : (
                    <span />
                  )}
                </span>
                <span className="mapping-nav-item__copy">
                  <strong>{status.field.label}</strong>
                  <small>
                    {status.sourceField
                      ? `来源：${status.sourceField}`
                      : status.hasDefault
                        ? "使用默认值"
                        : status.field.required
                          ? "尚未选择来源字段"
                          : "可选，尚未配置"}
                  </small>
                </span>
                <span className="mapping-nav-item__meta">
                  {status.dictionaryTotal > 0 && (
                    <small>
                      {status.dictionaryHandled}/{status.dictionaryTotal}
                    </small>
                  )}
                  <ArrowRight size={14} />
                </span>
              </button>
            ))}
          </section>
        ))}
        {!filteredStatuses.length && (
          <div className="mapping-navigator__empty">没有符合条件的字段</div>
        )}
      </div>
    </aside>
  );
}

export function DictionaryMappingEditor({
  dictionaryScopeLabel,
  dictionary,
  field,
  onAutoMap,
  onChange,
  onClear,
  onSaveSourceDictionary,
  rows,
  sourceDictionary,
  valueMappingsText,
}) {
  const [filter, setFilter] = useState("ALL");
  const [selectedMedicineGroup, setSelectedMedicineGroup] = useState(null);
  const [sourceDictionaryEditing, setSourceDictionaryEditing] = useState(false);
  if (!dictionary) return null;
  const handled = rows.filter((item) => item.matched || item.ignored).length;
  const filterCounts = Object.fromEntries(
    dictionaryFilters.map(({ key }) => [
      key,
      key === "ALL"
        ? rows.length
        : rows.filter((item) => dictionaryRowState(item) === key).length,
    ]),
  );
  const visibleRows = rows.filter(
    (item) => filter === "ALL" || dictionaryRowState(item) === filter,
  );
  return (
    <>
      <section className="dictionary-match-panel">
        <div className="dictionary-match-panel__heading">
          <div>
            <strong>二系列字典 → 新系统字典</strong>
            <span>
              {sourceDictionary?.name || "来源字段值"}
              {sourceDictionary?.entry
                ? ` · ${sourceDictionary.entry}.${sourceDictionary.keyField} → ${sourceDictionary.textField}`
                : ""}
              {` · 本次数据出现 ${rows.length} 类值${rows.some((item) => item.sourceIsBlank) ? "（含空值）" : ""} · 已处理 ${handled} 个`}
            </span>
          </div>
          <div>
            {sourceDictionary && onSaveSourceDictionary && (
              <button
                className="button button--secondary"
                onClick={() => setSourceDictionaryEditing(true)}
                type="button"
              >
                <PencilSimple size={16} />
                调整来源字典
              </button>
            )}
            <button
              className="button button--secondary"
              type="button"
              onClick={onAutoMap}
            >
              <LinkSimple size={16} />
              一键按含义匹配
            </button>
            <button
              className="button button--ghost"
              disabled={!valueMappingsText}
              onClick={onClear}
              type="button"
            >
              清空转换
            </button>
          </div>
        </div>
        {sourceDictionary?.loadStatus === "unavailable" && (
          <div className="dictionary-load-note dictionary-load-note--warning">
            <Warning size={16} weight="fill" />
            <span>{sourceDictionary.loadMessage}</span>
          </div>
        )}
        {sourceDictionary?.loadStatus === "empty" && (
          <div className="dictionary-load-note">
            <Info size={16} />
            <span>{sourceDictionary.loadMessage}</span>
          </div>
        )}
        <div className="dictionary-match-filters">
          <div aria-label="字典匹配状态筛选" role="group">
            {dictionaryFilters.map((item) => (
              <button
                aria-pressed={filter === item.key}
                className={filter === item.key ? "is-active" : ""}
                key={item.key}
                onClick={() => setFilter(item.key)}
                type="button"
              >
                {item.label}
                <b>{filterCounts[item.key]}</b>
              </button>
            ))}
          </div>
          <span>按来源出现顺序固定展示，操作后不重排</span>
        </div>
        <div className="dictionary-match-list">
          {visibleRows.map((item) => {
            const suggestionCode = item.suggestedTarget
              ? dictionaryItemValue(item.suggestedTarget)
              : "";
            const candidateCount = item.suggestedCandidates?.length || 0;
            const recommendedCodes = new Set(
              (item.suggestedCandidates || []).map(({ target }) =>
                dictionaryItemValue(target),
              ),
            );
            const recommendedOptions = (item.suggestedCandidates || []).map(
              ({ target, confidence, reason }) => ({
                value: dictionaryItemValue(target),
                label: `${target.text || target.na}（${dictionaryItemValue(target)}）`,
                description: `${reason} · 置信度 ${confidence}%`,
                keywords: `${target.py || ""} ${target.wb || ""}`,
                group: "推荐匹配（按置信度排序）",
              }),
            );
            const similarityOrderedTargets =
              item.targetSimilarityRanking ||
              dictionary.items.map((target, sourceOrder) => ({
                item: target,
                score: 0,
                reason: "",
                sourceOrder,
              }));
            return (
              <div className="dictionary-match-row" key={item.sourceValue}>
                <span className="dictionary-match-source">
                  <strong>
                    {item.sourceIsBlank
                      ? "空值"
                      : item.sourceText || "待补充来源含义"}
                  </strong>
                  <small className="dictionary-match-source__meta">
                    <span>
                      {item.sourceIsBlank
                        ? "来源为 NULL、空字符串或仅空格"
                        : `来源编码 ${item.sourceValue}`}
                    </span>
                    <button
                      aria-label={`查看${item.sourceIsBlank ? "空值" : item.sourceText || item.sourceValue}对应的 ${item.count} 条药品`}
                      onClick={() => setSelectedMedicineGroup(item)}
                      title="查看该来源值对应的药品明细"
                      type="button"
                    >
                      {item.count} 条药品 <ArrowRight size={12} />
                    </button>
                  </small>
                  {item.sourceProperties && (
                    <small title={item.sourceProperties}>
                      {item.sourceProperties}
                    </small>
                  )}
                </span>
                <ArrowRight size={17} />
                <SearchableSelect
                  ariaLabel={`${item.sourceIsBlank ? "空值" : item.sourceText || item.sourceValue}目标字典值`}
                  onChange={(next) => onChange(item.sourceValue, next)}
                  options={[
                    ...recommendedOptions,
                    {
                      value: "",
                      label: suggestionCode
                        ? `未采用 · 建议 ${item.suggestedTarget.text || item.suggestedTarget.na}（${suggestionCode}）`
                        : candidateCount
                          ? `尚未选择 · 有 ${candidateCount} 个相似候选`
                          : "尚未选择目标字典值",
                      description: suggestionCode
                        ? `${item.suggestionReason} · 置信度 ${item.suggestionConfidence}%`
                        : "",
                      group: "匹配操作",
                    },
                    ...(!field.required
                      ? [
                          {
                            value: IGNORE_VALUE_MAPPING_TARGET,
                            label: "忽略此来源值",
                            description:
                              "目标字段留空，并记录为已人工确认忽略；该值不再触发字典校验。",
                            group: "匹配操作",
                          },
                        ]
                      : []),
                    ...similarityOrderedTargets
                      .filter(
                        ({ item: targetItem }) =>
                          !recommendedCodes.has(
                            dictionaryItemValue(targetItem),
                          ),
                      )
                      .map(({ item: targetItem, score, reason }) => ({
                        value: dictionaryItemValue(targetItem),
                        label: `${targetItem.text || targetItem.na}（${dictionaryItemValue(targetItem)}）`,
                        keywords: `${targetItem.py || ""} ${targetItem.wb || ""}`,
                        description:
                          score > 0 ? `${reason} · 相似度 ${score}%` : "",
                        group: candidateCount
                          ? "其他字典项（按相似度排序）"
                          : "全部字典项（按相似度排序）",
                      })),
                  ]}
                  searchPlaceholder="按编码、名称或拼音查找"
                  value={
                    item.ignored
                      ? IGNORE_VALUE_MAPPING_TARGET
                      : item.appliedTarget
                  }
                />
                <span
                  className={`dictionary-match-status ${item.ignored ? "dictionary-match-status--ignored" : item.matched ? "dictionary-match-status--done" : candidateCount ? "dictionary-match-status--suggested" : ""}`}
                >
                  {item.ignored
                    ? "已忽略"
                    : item.matched
                      ? "已确认"
                      : suggestionCode
                        ? `${item.suggestionConfidence}% 建议`
                        : candidateCount
                          ? `${candidateCount} 个候选`
                          : "待确认"}
                </span>
              </div>
            );
          })}
          {!visibleRows.length && (
            <div className="dictionary-match-empty">
              {rows.length
                ? `当前没有${dictionaryFilters.find((item) => item.key === filter)?.label || "符合条件的"}项。`
                : "当前数据没有可配置的来源值。"}
            </div>
          )}
        </div>
      </section>
      {selectedMedicineGroup && (
        <SourceValueMedicineDialog
          field={field}
          item={selectedMedicineGroup}
          onClose={() => setSelectedMedicineGroup(null)}
        />
      )}
      {sourceDictionaryEditing && (
        <SourceDictionaryEditor
          dictionary={sourceDictionary}
          onClose={() => setSourceDictionaryEditing(false)}
          onSave={onSaveSourceDictionary}
          rows={rows}
          scopeLabel={dictionaryScopeLabel}
        />
      )}
    </>
  );
}

function SourceDictionaryEditor({
  dictionary,
  onClose,
  onSave,
  rows,
  scopeLabel,
}) {
  const [draft, setDraft] = useState(() =>
    (dictionary.items || []).map((item) => ({ ...item })),
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const presentCodes = useMemo(
    () =>
      rows
        .filter((item) => !item.sourceIsBlank)
        .map((item) => `${item.sourceValue}`.trim())
        .filter(Boolean),
    [rows],
  );
  const configuredCodes = new Set(
    draft.map((item) => `${item.key ?? ""}`.trim()).filter(Boolean),
  );
  const missingCodes = presentCodes.filter(
    (code, index) =>
      !configuredCodes.has(code) && presentCodes.indexOf(code) === index,
  );

  useEffect(() => {
    const closeOnEscape = (event) => {
      if (event.key === "Escape" && !saving) onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose, saving]);

  const updateItem = (index, key, value) => {
    setDraft((current) =>
      current.map((item, itemIndex) =>
        itemIndex === index ? { ...item, [key]: value } : item,
      ),
    );
  };
  const validate = () => {
    const normalized = draft.map((item) => ({
      ...item,
      key: `${item.key ?? ""}`.trim(),
      text: `${item.text ?? ""}`.trim(),
      properties: item.properties || {},
    }));
    if (normalized.some((item) => !item.key || !item.text)) {
      throw new Error("每个字典项都需要填写来源编码和中文含义");
    }
    const keys = normalized.map((item) => item.key);
    if (new Set(keys).size !== keys.length) {
      throw new Error("来源编码不能重复，请合并重复项后再保存");
    }
    return normalized;
  };
  const save = async (items) => {
    setError("");
    setSaving(true);
    try {
      await onSave(items);
      onClose();
    } catch (nextError) {
      setError(nextError?.message || `${nextError}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      className="dictionary-medicine-backdrop"
      onMouseDown={() => !saving && onClose()}
      role="presentation"
    >
      <section
        aria-label={`调整${dictionary.name}来源字典`}
        aria-modal="true"
        className="source-dictionary-dialog"
        onMouseDown={(event) => event.stopPropagation()}
        role="dialog"
      >
        <header className="source-dictionary-dialog__header">
          <div>
            <span className="eyebrow">二系列phis · 项目级配置</span>
            <h2>调整“{dictionary.name}”来源字典</h2>
            <p>
              仅修正老系统编码的实际含义；保存后将重新参与按含义推荐、预览和校验。
            </p>
          </div>
          <button
            aria-label="关闭来源字典调整"
            className="icon-button"
            disabled={saving}
            onClick={onClose}
            type="button"
          >
            <X size={19} />
          </button>
        </header>
        <div className="source-dictionary-dialog__scope">
          <strong>适用项目</strong>
          <span>{scopeLabel}</span>
          <b>{dictionary.customized ? "已人工调整" : "当前为默认定义"}</b>
        </div>
        <div className="source-dictionary-dialog__toolbar">
          <span>
            当前 {draft.length} 项
            {missingCodes.length
              ? ` · 本批还有 ${missingCodes.length} 个编码未定义`
              : " · 本批出现的编码均已覆盖"}
          </span>
          <div>
            {missingCodes.length > 0 && (
              <button
                className="button button--secondary"
                onClick={() =>
                  setDraft((current) => [
                    ...current,
                    ...missingCodes.map((key) => ({
                      key,
                      text: "",
                      properties: {},
                    })),
                  ])
                }
                type="button"
              >
                <Plus size={15} /> 补齐本批编码
              </button>
            )}
            <button
              className="button button--secondary"
              onClick={() =>
                setDraft((current) => [
                  ...current,
                  { key: "", text: "", properties: {} },
                ])
              }
              type="button"
            >
              <Plus size={15} /> 新增字典项
            </button>
          </div>
        </div>
        <div className="source-dictionary-table">
          <div className="source-dictionary-table__head">
            <span>来源编码</span>
            <span>项目实际中文含义</span>
            <span>操作</span>
          </div>
          {draft.map((item, index) => (
            <div className="source-dictionary-table__row" key={`${index}-${item.key}`}>
              <input
                aria-label={`第 ${index + 1} 项来源编码`}
                onChange={(event) => updateItem(index, "key", event.target.value)}
                placeholder="例如 3"
                value={item.key}
              />
              <input
                aria-label={`第 ${index + 1} 项中文含义`}
                onChange={(event) => updateItem(index, "text", event.target.value)}
                placeholder="例如 喹诺酮类"
                value={item.text}
              />
              <button
                aria-label={`删除第 ${index + 1} 项`}
                className="icon-button icon-button--danger"
                onClick={() =>
                  setDraft((current) =>
                    current.filter((_, itemIndex) => itemIndex !== index),
                  )
                }
                type="button"
              >
                <Trash size={16} />
              </button>
            </div>
          ))}
          {!draft.length && (
            <div className="source-dictionary-table__empty">
              当前没有字典项，可新增或补齐本批实际编码。
            </div>
          )}
        </div>
        {error && <div className="source-dictionary-dialog__error">{error}</div>}
        <footer className="source-dictionary-dialog__footer">
          <button
            className="button button--ghost"
            disabled={saving || !dictionary.customized}
            onClick={() => save(null)}
            type="button"
          >
            <ArrowCounterClockwise size={16} /> 恢复默认定义
          </button>
          <div>
            <button
              className="button button--secondary"
              disabled={saving}
              onClick={onClose}
              type="button"
            >
              取消
            </button>
            <button
              className="button button--primary"
              disabled={saving}
              onClick={() => {
                try {
                  save(validate());
                } catch (nextError) {
                  setError(nextError.message);
                }
              }}
              type="button"
            >
              <FloppyDisk size={16} />
              {saving ? "正在保存…" : "保存项目调整"}
            </button>
          </div>
        </footer>
      </section>
    </div>
  );
}
