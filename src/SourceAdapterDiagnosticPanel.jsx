import { DownloadSimple, Warning } from "@phosphor-icons/react";
import { SourceStructureCompatibility } from "./SourceStructureCompatibility";

export function SourceAdapterDiagnosticPanel({
  diagnostics = [],
  inspection,
  onExportReport,
  onExportSupportPackage,
  title = "当前项目结构差异",
}) {
  if (!inspection) return null;
  const checkedObjects = inspection.checkedObjects || [];
  const missingObjects = inspection.missingObjects || [];
  const warnings = inspection.warnings || [];

  return (
    <div className="adapter-diagnostic">
      <div className="adapter-diagnostic__heading">
        <div>
          <strong>{title}</strong>
          <span>
            已核对 {checkedObjects.length} 个对象
            {missingObjects.length
              ? `，缺少 ${missingObjects.length} 个`
              : "，未发现必需对象缺失"}
          </span>
        </div>
        <div className="adapter-diagnostic__actions">
          <button type="button" onClick={onExportReport}>
            <DownloadSimple size={15} />
            当前报告
          </button>
          <button type="button" onClick={onExportSupportPackage}>
            <DownloadSimple size={15} />
            导出支持包
          </button>
        </div>
      </div>
      {missingObjects.length > 0 && (
        <div className="adapter-diagnostic__missing">
          <Warning size={16} weight="fill" />
          <span>
            <strong>缺失或不可访问：</strong>
            {missingObjects.join("、")}
          </span>
        </div>
      )}
      <SourceStructureCompatibility compatibility={inspection.compatibility} />
      <details>
        <summary>查看已核对来源对象和完整提醒</summary>
        <div className="adapter-diagnostic__details">
          <div>
            <strong>已核对对象</strong>
            <span>{checkedObjects.join("、") || "无"}</span>
          </div>
          <div>
            <strong>适配器提醒</strong>
            <span>{warnings.join("；") || "无"}</span>
          </div>
        </div>
      </details>
      {diagnostics.length > 0 && (
        <details className="adapter-diagnostic-history">
          <summary>本地诊断历史（{diagnostics.length} 次结构变化）</summary>
          <div className="adapter-diagnostic-history__list">
            {diagnostics.slice(0, 8).map((record) => (
              <div key={record.diagnosticId}>
                <span className={record.detected ? "is-ready" : "is-blocked"}>
                  {record.detected ? "通过" : "未通过"}
                </span>
                <strong>
                  {record.schema || "未识别 Schema"} · v{record.adapterVersion}
                </strong>
                <small>
                  {new Date(record.recordedAt).toLocaleString("zh-CN", {
                    hour12: false,
                  })}
                  {record.missingObjects.length
                    ? ` · 缺少 ${record.missingObjects.length} 个对象`
                    : " · 无必需对象缺失"}
                </small>
              </div>
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
