import { useMemo, useState } from "react";
import {
  ArrowRight,
  CheckCircle,
  Info,
  LinkSimple,
  MagnifyingGlass,
  Warning,
} from "@phosphor-icons/react";
import { dictionaryItemValue } from "./dictionary";
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
  dictionary,
  field,
  onAutoMap,
  onChange,
  onClear,
  rows,
  sourceDictionary,
  valueMappingsText,
}) {
  const [filter, setFilter] = useState("ALL");
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
          return (
            <div className="dictionary-match-row" key={item.sourceValue}>
              <span className="dictionary-match-source">
                <strong>
                  {item.sourceIsBlank
                    ? "空值"
                    : item.sourceText || "待补充来源含义"}
                </strong>
                <small>
                  {item.sourceIsBlank
                    ? `来源为 NULL、空字符串或仅空格 · ${item.count} 条药品`
                    : `来源编码 ${item.sourceValue} · ${item.count} 条药品`}
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
                  ...dictionary.items
                    .filter(
                      (targetItem) =>
                        !recommendedCodes.has(dictionaryItemValue(targetItem)),
                    )
                    .map((targetItem) => ({
                      value: dictionaryItemValue(targetItem),
                      label: `${targetItem.text || targetItem.na}（${dictionaryItemValue(targetItem)}）`,
                      keywords: `${targetItem.py || ""} ${targetItem.wb || ""}`,
                      group: "其他字典项（保持原顺序）",
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
  );
}
