import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";

const manifestUrl = new URL("../src-tauri/driver-packs/manifest.json", import.meta.url);
const driverDirectoryUrl = new URL("../src-tauri/driver-packs/", import.meta.url);

async function loadManifest() {
  return JSON.parse(await readFile(manifestUrl, "utf8"));
}

test("数据库交付清单覆盖约定的数据库家族并优先 Windows x64", async () => {
  const packs = await loadManifest();
  const expectedKinds = [
    "mysql",
    "oracle",
    "dameng",
    "postgresql",
    "opengauss",
    "vastbase",
    "gbase8c",
    "gbase8a",
    "gbase8s",
    "kingbase",
  ];

  assert.deepEqual(
    [...new Set(packs.map((pack) => pack.databaseKind))].sort(),
    expectedKinds.sort(),
  );
  assert.equal(new Set(packs.map((pack) => pack.id)).size, packs.length, "驱动包 ID 必须唯一");

  for (const pack of packs) {
    assert.equal(pack.platformPriority?.[0], "windows-x64", `${pack.id} 未把 Windows x64 放在首位`);
    assert.match(pack.officialUrl ?? "", /^https:\/\//, `${pack.id} 缺少 HTTPS 官方入口`);
    assert.ok(pack.installGuide?.trim().length >= 12, `${pack.id} 缺少可执行的安装说明`);
  }
});

test("PG 兼容家族内置通用协议，8a/8s 不会误归入 PG", async () => {
  const packs = await loadManifest();
  const byKind = Object.fromEntries(packs.map((pack) => [pack.databaseKind, pack]));

  for (const kind of ["postgresql", "opengauss", "vastbase", "gbase8c", "kingbase"]) {
    assert.equal(byKind[kind].delivery, "bundled", `${kind} 应使用应用内置协议`);
    assert.equal(byKind[kind].protocol, "postgresql-wire", `${kind} 协议声明错误`);
    assert.equal(byKind[kind].defaultDriver, "", `${kind} 默认不应依赖 ODBC 驱动名`);
  }
  for (const kind of ["gbase8a", "gbase8s"]) {
    assert.equal(byKind[kind].delivery, "vendor-install", `${kind} 应由厂商交付驱动`);
    assert.equal(byKind[kind].protocol, "odbc", `${kind} 不应走 PostgreSQL 通用协议`);
  }
});

test("受许可约束的厂商二进制不会被误装入驱动目录", async () => {
  const entries = await readdir(driverDirectoryUrl, { recursive: true });
  const forbidden = entries.filter((name) => /\.(dll|exe|msi|dylib|so(?:\.\d+)*)$/i.test(name));
  assert.deepEqual(forbidden, [], `发现不应随包分发的厂商二进制：${forbidden.join(", ")}`);
});

test("Windows 发布流程执行驱动检查并产出 NSIS 安装包", async () => {
  const workflow = await readFile(
    new URL("../.github/workflows/windows-build.yml", import.meta.url),
    "utf8",
  );
  assert.match(workflow, /runs-on:\s*windows-latest/);
  assert.match(workflow, /npm run test:drivers/);
  assert.match(workflow, /bundle\/nsis\/\*\.exe/);
  assert.match(workflow, /if-no-files-found:\s*error/);
  assert.match(workflow, /verify-windows-installer\.ps1/);
  assert.match(workflow, /windows-install-acceptance\.json/);
});

test("macOS 发布流程强制签名、公证和票据验收", async () => {
  const workflow = await readFile(
    new URL("../.github/workflows/macos-release.yml", import.meta.url),
    "utf8",
  );
  for (const secret of [
    "APPLE_CERTIFICATE",
    "APPLE_CERTIFICATE_PASSWORD",
    "APPLE_SIGNING_IDENTITY",
    "APPLE_ID",
    "APPLE_PASSWORD",
    "APPLE_TEAM_ID",
  ]) {
    assert.match(workflow, new RegExp(secret));
  }
  assert.match(workflow, /REQUIRE_NOTARIZATION:\s*"1"/);
  assert.match(workflow, /verify-macos-release\.sh/);
  assert.match(workflow, /macos-release-acceptance\.json/);
});
