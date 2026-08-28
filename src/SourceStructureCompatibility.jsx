import { CheckCircle, Warning } from "@phosphor-icons/react";

function CompatibilityItem({ item }) {
  return (
    <div>
      <strong>{item.path}</strong>
      <span>
        {item.message}
        {item.usedBy?.length ? `；影响：${item.usedBy.join("、")}` : ""}
      </span>
      {item.fallback && <small>{item.fallback}</small>}
    </div>
  );
}

export function SourceStructureCompatibility({ compatibility }) {
  if (!compatibility || compatibility.status === "UNAVAILABLE") return null;
  const status = `${compatibility.status || "REVIEW"}`.toUpperCase();
  const details = [
    ...(compatibility.reviews || []),
    ...(compatibility.compatibleFallbacks || []),
  ];
  return (
    <div
      className={`adapter-compatibility adapter-compatibility--${status.toLowerCase()}`}
    >
      <div className="adapter-compatibility__summary">
        {status === "COMPATIBLE" ? (
          <CheckCircle size={17} weight="fill" />
        ) : (
          <Warning size={17} weight="fill" />
        )}
        <div>
          <strong>
            {status === "BLOCKED"
              ? "当前结构存在阻断项"
              : status === "REVIEW"
                ? "核心结构可读，仍需核对"
                : "当前结构可兼容"}
          </strong>
          <span>{compatibility.message}</span>
        </div>
        <div className="adapter-compatibility__counts">
          <span className="is-blocked">
            阻断：{compatibility.blockers?.length || 0}
          </span>
          <span className="is-review">
            核对：{compatibility.reviews?.length || 0}
          </span>
          <span className="is-fallback">
            降级：{compatibility.compatibleFallbacks?.length || 0}
          </span>
        </div>
      </div>
      {compatibility.blockers?.length > 0 && (
        <div className="adapter-compatibility__blockers">
          {compatibility.blockers.map((item, index) => (
            <CompatibilityItem key={`${item.path}-${index}`} item={item} />
          ))}
        </div>
      )}
      {details.length > 0 && (
        <details className="adapter-compatibility__details">
          <summary>查看需核对项和安全降级说明</summary>
          <div>
            {details.map((item, index) => (
              <CompatibilityItem key={`${item.path}-${index}`} item={item} />
            ))}
          </div>
        </details>
      )}
    </div>
  );
}
