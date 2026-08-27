#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
app_path="${1:-}"
if [[ -z "$app_path" ]]; then
  app_path="$(find "$repo_root/src-tauri/target/release/bundle/macos" -maxdepth 1 -name '*.app' -print -quit)"
fi
if [[ -z "$app_path" || ! -d "$app_path" ]]; then
  echo "没有找到 macOS .app 产物" >&2
  exit 1
fi

codesign --verify --deep --strict --verbose=2 "$app_path"
bundle_id="$(defaults read "$app_path/Contents/Info.plist" CFBundleIdentifier)"
if [[ "$bundle_id" != "cn.bsoft.rbmh.medicine-migration" ]]; then
  echo "Bundle Identifier 不正确：$bundle_id" >&2
  exit 1
fi
if [[ ! -f "$app_path/Contents/Resources/icon.icns" ]]; then
  echo "应用包缺少 icon.icns" >&2
  exit 1
fi

signature="$(codesign -dv --verbose=4 "$app_path" 2>&1)"
authority="$(printf '%s\n' "$signature" | awk -F= '/^Authority=/{print $2; exit}')"
notarized=false
if [[ "${REQUIRE_NOTARIZATION:-0}" == "1" ]]; then
  if [[ "$authority" != Developer\ ID\ Application:* ]]; then
    echo "正式发布必须使用 Developer ID Application 签名，当前为：${authority:-未知}" >&2
    exit 1
  fi
  spctl --assess --type execute --verbose=4 "$app_path"
  xcrun stapler validate "$app_path"
  notarized=true
fi

mkdir -p "$repo_root/artifacts"
cat > "$repo_root/artifacts/macos-release-acceptance.json" <<JSON
{
  "app": "${app_path}",
  "bundleId": "${bundle_id}",
  "signingAuthority": "${authority}",
  "notarizationRequired": $([[ "${REQUIRE_NOTARIZATION:-0}" == "1" ]] && echo true || echo false),
  "notarizationValidated": ${notarized}
}
JSON
cat "$repo_root/artifacts/macos-release-acceptance.json"
