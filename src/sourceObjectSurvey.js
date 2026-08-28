import { sourceObjectOptions } from "./sourceDatabaseObjects.js";
import { sourceObjectSuitability } from "./sourceFieldMatching.js";

export function sourceObjectSurveyCandidates(objects, maximum = 5) {
  return sourceObjectOptions(objects)
    .filter((option) => Number(option.data?.medicineClueScore || 0) > 0)
    .slice(0, Math.max(0, maximum));
}
export function sourceObjectSurveyResult({
  option,
  sourceResult,
  targetFields,
} = {}) {
  const preview = sourceResult?.preview;
  if (!preview?.columns?.length) {
    return {
      objectName: option?.value || "",
      medicineClueScore: Number(option?.data?.medicineClueScore || 0),
      status: "FAILED",
      title: "未能读取有效字段",
      description: "可稍后单独预览，或检查该对象的只读权限。",
      foundCount: 0,
      clues: [],
      columnCount: 0,
      sampledRowCount: 0,
    };
  }
  const suitability = sourceObjectSuitability({
    columns: preview.columns,
    columnMetadata: preview.columnMetadata,
    rows: preview.rows,
    targetFields,
  });
  return {
    objectName: option.value,
    medicineClueScore: Number(option.data?.medicineClueScore || 0),
    status: "READY",
    tone: suitability.tone,
    title: suitability.title,
    description: suitability.description,
    foundCount: suitability.foundCount,
    hasMedicineName: suitability.clues.some(
      (clue) => clue.key === "naMed" && clue.column && clue.hasSample,
    ),
    clues: suitability.clues,
    columnCount: preview.columns.length,
    sampledRowCount: preview.rows?.length || 0,
    metadataMessage: sourceResult.metadataMessage || "",
  };
}

export function failedSourceObjectSurveyResult(option) {
  return {
    objectName: option?.value || "",
    medicineClueScore: Number(option?.data?.medicineClueScore || 0),
    status: "FAILED",
    title: "候选样例读取失败",
    description: "未纳入推荐排序；可稍后单独预览以查看具体错误。",
    foundCount: 0,
    clues: [],
    columnCount: 0,
    sampledRowCount: 0,
  };
}

export function rankSourceObjectSurveyResults(results) {
  return [...(results || [])].sort((left, right) => {
    const leftReady = left.status === "READY" ? 1 : 0;
    const rightReady = right.status === "READY" ? 1 : 0;
    const leftName = left.hasMedicineName ? 1 : 0;
    const rightName = right.hasMedicineName ? 1 : 0;
    return (
      rightReady - leftReady ||
      rightName - leftName ||
      Number(right.foundCount || 0) - Number(left.foundCount || 0) ||
      Number(right.medicineClueScore || 0) - Number(left.medicineClueScore || 0) ||
      `${left.objectName}`.localeCompare(`${right.objectName}`)
    );
  });
}
