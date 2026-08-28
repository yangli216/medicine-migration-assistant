function normalized(value) {
  return `${value ?? ""}`.trim().toLowerCase();
}

export function normalizeSourceQueryIdentity(value) {
  return `${value ?? ""}`.trim().replace(/;+\s*$/, "");
}

function stableSourceFingerprint(value) {
  const text = `${value}`;
  let first = 2166136261;
  let second = 2246822507;
  for (const character of text) {
    const code = character.codePointAt(0);
    first ^= code;
    first = Math.imul(first, 16777619) >>> 0;
    second ^= code;
    second = Math.imul(second, 3266489909) >>> 0;
  }
  return `${first.toString(16).padStart(8, "0")}${second
    .toString(16)
    .padStart(8, "0")}-${text.length}`;
}

function sanitizedConnectionConfiguration(value) {
  const text = `${value || ""}`.trim();
  if (!text) return "";
  const parts = [];
  let current = "";
  let braceDepth = 0;
  for (const character of text) {
    if (character === "{") braceDepth += 1;
    if (character === "}" && braceDepth > 0) braceDepth -= 1;
    if (character === ";" && braceDepth === 0) {
      parts.push(current);
      current = "";
    } else {
      current += character;
    }
  }
  if (current.trim()) parts.push(current);
  const credentialKeys = new Set([
    "pwd",
    "password",
    "uid",
    "user",
    "userid",
    "username",
  ]);
  return parts
    .map((part) => {
      const separator = part.indexOf("=");
      if (separator < 0) return "";
      const key = normalized(part.slice(0, separator)).replace(/\s+/g, "");
      const value = normalized(part.slice(separator + 1));
      return key && value && !credentialKeys.has(key) ? `${key}=${value}` : "";
    })
    .filter(Boolean)
    .sort()
    .join(";");
}

export function sourceSelectionIdentity({
  sourceObject = "",
  sourceQuery = "",
} = {}) {
  const object = `${sourceObject || ""}`.trim();
  if (object)
    return `object:${stableSourceFingerprint(object)}:${normalized(object).slice(0, 80)}`;
  const query = normalizeSourceQueryIdentity(sourceQuery);
  return query ? `query:${stableSourceFingerprint(query)}` : "";
}

export function sourceMappingProfileForSelection({
  scopedProfile = null,
  legacyProfile = null,
  resolvedQuery = "",
} = {}) {
  if (scopedProfile) return scopedProfile;
  const legacyQuery = normalizeSourceQueryIdentity(legacyProfile?.sourceQuery);
  const currentQuery = normalizeSourceQueryIdentity(resolvedQuery);
  return legacyQuery && legacyQuery === currentQuery ? legacyProfile : null;
}

export function legacySourceMappingPromotionRequest({
  scopedScope = null,
  scopedProfile = null,
  legacyProfile = null,
  resolvedQuery = "",
} = {}) {
  if (!scopedScope || scopedProfile || !legacyProfile) return null;
  if (
    sourceMappingProfileForSelection({ legacyProfile, resolvedQuery }) !==
    legacyProfile
  )
    return null;
  return {
    scope: scopedScope,
    adapterVersion: Number(legacyProfile.adapterVersion || 0),
    sourceKey: legacyProfile.sourceKey || "",
    sourceQuery: normalizeSourceQueryIdentity(resolvedQuery),
    mapping: legacyProfile.mapping || {},
    rules: legacyProfile.rules || {},
    dictionaryOverrides: legacyProfile.dictionaryOverrides || {},
  };
}

export function databaseSourceIdentity(profile = {}) {
  const kind = normalized(profile.kind || "database");
  const host = normalized(profile.host);
  const port = `${profile.port || ""}`.trim();
  const database = normalized(profile.serviceName || profile.database);
  const schema = normalized(profile.schema || profile.username);
  if (host || database || schema)
    return `${kind}:${host}:${port}/${database}:${schema}`;
  const configuration = sanitizedConnectionConfiguration(
    profile.connectionString,
  );
  const fallback = configuration || normalized(profile.driver);
  return fallback
    ? `${kind}:configuration:${stableSourceFingerprint(fallback)}`
    : "";
}

export function sourceMappingProfileScope({
  adapterId,
  fileName = "",
  profile = {},
  sourceObject = "",
  sourceQuery = "",
  sourceMode = "database",
  targetTenantId,
} = {}) {
  const adapter = `${adapterId || ""}`.trim().toUpperCase();
  const tenant = normalized(targetTenantId);
  if (!adapter || !tenant) return null;
  const normalizedFileName = normalized(fileName);
  if (sourceMode === "file" && !normalizedFileName) return null;
  if (
    sourceMode === "database" &&
    (!normalized(profile.kind) || !databaseSourceIdentity(profile))
  )
    return null;
  let sourceIdentity =
    sourceMode === "file"
      ? `file:${normalizedFileName}`
      : `database:${databaseSourceIdentity(profile)}`;
  if (sourceMode === "database") {
    const selection = sourceSelectionIdentity({ sourceObject, sourceQuery });
    if (selection) {
      const databaseIdentity = databaseSourceIdentity(profile);
      sourceIdentity = `database-selection:${stableSourceFingerprint(databaseIdentity)}|${selection}`;
    }
  }
  return {
    adapterId: adapter,
    sourceIdentity,
    targetTenantId: tenant,
  };
}

export function sourceAdapterId({ phis27 = false, sourceMode = "database" } = {}) {
  if (phis27) return "PHIS27";
  return sourceMode === "file" ? "GENERIC_FILE" : "GENERIC_DATABASE";
}
