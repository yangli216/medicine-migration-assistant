import { Plus, Trash } from "@phosphor-icons/react";
import { SearchableSelect } from "./SearchableSelect";

const CONDITION_OPTIONS = [
  { value: "EMPTY", label: "为空" },
  { value: "NOT_EMPTY", label: "不为空" },
  { value: "EQUALS", label: "等于" },
  { value: "NOT_EQUALS", label: "不等于" },
  { value: "CONTAINS", label: "包含" },
];

const CONDITION_ELSE_OPTIONS = [
  { value: "KEEP", label: "不满足时保留原值" },
  { value: "EMPTY", label: "不满足时留空" },
  { value: "DEFAULT", label: "不满足时使用默认值" },
];

function advancedSummary(rule) {
  const summaries = [];
  const sourceCount = 1 + (rule.additionalSourceFields?.length || 0);
  if (sourceCount > 1) summaries.push(`组合 ${sourceCount} 个字段`);
  if (Number(rule.maxLength) > 0) {
    summaries.push(
      `${rule.truncateMode === "KEEP_END" ? "保留末尾" : "保留开头"} ${rule.maxLength} 字`,
    );
  }
  if (rule.conditionOperator && rule.conditionOperator !== "ALWAYS") {
    summaries.push("有条件执行");
  }
  return summaries.join(" · ") || "未配置";
}

export function FieldAdvancedRuleEditor({
  fieldLabel,
  primarySourceField,
  rule = {},
  sourceOptions,
  onChange,
}) {
  const additionalSourceFields = rule.additionalSourceFields || [];
  const conditionEnabled = Boolean(
    rule.conditionOperator && rule.conditionOperator !== "ALWAYS",
  );
  const update = (patch) => onChange({ ...rule, ...patch });
  const usedFields = new Set([primarySourceField, ...additionalSourceFields]);

  return (
    <details className="advanced-rule-editor">
      <summary>
        <span>高级取值（组合、截取、条件）</span>
        <small>{advancedSummary(rule)}</small>
      </summary>
      <div className="advanced-rule-editor__body">
        <section>
          <div className="advanced-rule-editor__section-head">
            <span>
              <strong>组合来源</strong>
              <small>按显示顺序拼接，空字段会自动略过</small>
            </span>
            <button
              type="button"
              className="button button--ghost advanced-rule-editor__add"
              disabled={!primarySourceField || additionalSourceFields.length >= 3}
              onClick={() =>
                update({
                  additionalSourceFields: [...additionalSourceFields, ""],
                })
              }
            >
              <Plus size={14} />
              添加来源
            </button>
          </div>
          {additionalSourceFields.length > 0 && (
            <div className="advanced-rule-editor__sources">
              <span className="advanced-rule-editor__primary">
                主字段：{primarySourceField || "尚未选择"}
              </span>
              {additionalSourceFields.map((sourceField, index) => (
                <div key={`${index}-${sourceField}`}>
                  <SearchableSelect
                    ariaLabel={`${fieldLabel}组合来源${index + 2}`}
                    value={sourceField}
                    onChange={(next) =>
                      update({
                        additionalSourceFields: additionalSourceFields.map(
                          (item, itemIndex) =>
                            itemIndex === index ? next : item,
                        ),
                      })
                    }
                    options={sourceOptions.map((option) => ({
                      ...option,
                      disabled:
                        usedFields.has(option.value) && option.value !== sourceField,
                    }))}
                    searchPlaceholder="按字段名、注释或表名过滤"
                  />
                  <button
                    type="button"
                    className="icon-button"
                    aria-label={`移除组合来源${index + 2}`}
                    onClick={() =>
                      update({
                        additionalSourceFields: additionalSourceFields.filter(
                          (_, itemIndex) => itemIndex !== index,
                        ),
                      })
                    }
                  >
                    <Trash size={16} />
                  </button>
                </div>
              ))}
              <label>
                <span>字段之间</span>
                <input
                  maxLength={8}
                  placeholder="不加连接符"
                  value={rule.joinSeparator || ""}
                  onChange={(event) =>
                    update({ joinSeparator: event.target.value })
                  }
                />
              </label>
            </div>
          )}
        </section>

        <section>
          <div className="advanced-rule-editor__section-head">
            <span>
              <strong>长度限制</strong>
              <small>仅在目标字段有长度限制时设置，0 表示不截取</small>
            </span>
          </div>
          <div className="advanced-rule-editor__truncate">
            <input
              aria-label={`${fieldLabel}最大长度`}
              min="0"
              max="4000"
              placeholder="最大字符数"
              type="number"
              value={rule.maxLength || ""}
              onChange={(event) =>
                update({
                  maxLength: Math.min(
                    4000,
                    Math.max(0, Number(event.target.value) || 0),
                  ),
                })
              }
            />
            <SearchableSelect
              ariaLabel={`${fieldLabel}截取方向`}
              value={rule.truncateMode || "KEEP_START"}
              onChange={(next) => update({ truncateMode: next })}
              options={[
                { value: "KEEP_START", label: "超长时保留开头" },
                { value: "KEEP_END", label: "超长时保留末尾" },
              ]}
              searchPlaceholder="选择截取方式"
            />
          </div>
        </section>

        <section>
          <div className="advanced-rule-editor__section-head">
            <span>
              <strong>按条件执行</strong>
              <small>只支持一条条件，避免规则难以核对</small>
            </span>
            {!conditionEnabled && (
              <button
                type="button"
                className="button button--ghost advanced-rule-editor__add"
                onClick={() =>
                  update({
                    conditionField: primarySourceField || sourceOptions[0]?.value || "",
                    conditionOperator: "NOT_EMPTY",
                    conditionValue: "",
                    conditionElse: "KEEP",
                  })
                }
              >
                <Plus size={14} />
                添加条件
              </button>
            )}
          </div>
          {conditionEnabled && (
            <div className="advanced-rule-editor__condition">
              <span>当</span>
              <SearchableSelect
                ariaLabel={`${fieldLabel}条件字段`}
                value={rule.conditionField || ""}
                onChange={(next) => update({ conditionField: next })}
                options={sourceOptions}
                searchPlaceholder="选择判断字段"
              />
              <SearchableSelect
                ariaLabel={`${fieldLabel}条件关系`}
                value={rule.conditionOperator}
                onChange={(next) => update({ conditionOperator: next })}
                options={CONDITION_OPTIONS}
                searchPlaceholder="选择条件"
              />
              {!['EMPTY', 'NOT_EMPTY'].includes(rule.conditionOperator) && (
                <input
                  aria-label={`${fieldLabel}条件值`}
                  placeholder="比较值"
                  value={rule.conditionValue || ""}
                  onChange={(event) =>
                    update({ conditionValue: event.target.value })
                  }
                />
              )}
              <SearchableSelect
                ariaLabel={`${fieldLabel}条件不满足时`}
                value={rule.conditionElse || "KEEP"}
                onChange={(next) => update({ conditionElse: next })}
                options={CONDITION_ELSE_OPTIONS}
                searchPlaceholder="选择未命中处理"
              />
              <button
                type="button"
                className="icon-button"
                aria-label={`移除${fieldLabel}条件`}
                onClick={() =>
                  update({
                    conditionField: "",
                    conditionOperator: "ALWAYS",
                    conditionValue: "",
                    conditionElse: "KEEP",
                  })
                }
              >
                <Trash size={16} />
              </button>
            </div>
          )}
        </section>
      </div>
    </details>
  );
}
