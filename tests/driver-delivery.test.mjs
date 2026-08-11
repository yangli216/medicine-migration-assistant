import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";

const manifestUrl = new URL("../src-tauri/driver-packs/manifest.json", import.meta.url);
const driverDirectoryUrl = new URL("../src-tauri/driver-packs/", import.meta.url);
const windowsConfigUrl = new URL("../src-tauri/tauri.windows.conf.json", import.meta.url);

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

test("Oracle Windows x64 使用固定校验值准备并自动登记内置驱动", async () => {
  const packs = await loadManifest();
  const oracle = packs.find((pack) => pack.databaseKind === "oracle");
  assert.equal(oracle.delivery, "licensed-bundle");
  assert.equal(oracle.protocol, "odbc");
  assert.equal(oracle.version, "19.31 · Windows x64");
  assert.equal(oracle.defaultDriver, "Oracle 19 ODBC driver");

  const prepareScript = await readFile(
    new URL("../scripts/prepare-oracle-windows-driver.mjs", import.meta.url),
    "utf8",
  );
  assert.match(prepareScript, /download\.oracle\.com\/otn_software\/nt\/instantclient\/1931000/);
  assert.match(prepareScript, /instantclient-basic-windows\.x64-19\.31\.0\.0\.0dbru\.zip/);
  assert.match(prepareScript, /instantclient-odbc-windows\.x64-19\.31\.0\.0\.0dbru\.zip/);
  assert.match(prepareScript, /9e990e02e936fe073f05b7cb9bb95bd5163372083826e9321a9d85bff88e829d/);
  assert.match(prepareScript, /cdda3ba6bd47b7413eaf7b35b4fec903101a6e0623ce7a95724e8a3437c407d7/);
  for (const fileName of ["BASIC_LICENSE", "ODBC_LICENSE", "sqora32.dll", "odbc_install.exe"]) {
    assert.match(prepareScript, new RegExp(fileName.replace(".", "\\.")));
  }

  const windowsConfig = JSON.parse(await readFile(windowsConfigUrl, "utf8"));
  const nsis = windowsConfig.bundle.windows.nsis;
  assert.equal(nsis.installMode, "perMachine");
  assert.equal(nsis.installerHooks, "windows/oracle-driver-hooks.nsh");
  assert.ok(
    Object.keys(windowsConfig.bundle.resources).some((entry) =>
      entry.includes("instantclient_19_31_bsoft_migration"),
    ),
  );

  const hooks = await readFile(
    new URL("../src-tauri/windows/oracle-driver-hooks.nsh", import.meta.url),
    "utf8",
  );
  assert.match(hooks, /NSIS_HOOK_POSTINSTALL/);
  assert.match(hooks, /SetRegView 64/);
  assert.match(hooks, /WriteRegStr HKLM/);
  assert.match(hooks, /\$INSTDIR\\oracle\\instantclient_19_31_bsoft_migration/);
  assert.match(hooks, /sqora32\.dll/);
  assert.match(hooks, /NSIS_HOOK_PREUNINSTALL/);
  assert.match(hooks, /DeleteRegKey HKLM/);
});

test("Windows 发布流程执行驱动检查并产出 NSIS 安装包", async () => {
  const workflow = await readFile(
    new URL("../.github/workflows/windows-build.yml", import.meta.url),
    "utf8",
  );
  assert.match(workflow, /runs-on:\s*windows-latest/);
  assert.match(workflow, /npm run prepare:oracle-driver/);
  assert.match(workflow, /npm run verify:oracle-driver/);
  assert.match(workflow, /verify-oracle-windows-driver-tools\.ps1/);
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
