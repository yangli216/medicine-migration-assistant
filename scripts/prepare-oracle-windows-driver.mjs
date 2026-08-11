import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import {
  access,
  cp,
  mkdir,
  readFile,
  readdir,
  rm,
  writeFile,
} from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const VERSION = "19.31.0.0.0";
const ARCHIVE_DIRECTORY = "instantclient_19_31";
const BUNDLE_DIRECTORY = "instantclient_19_31_bsoft_migration";
const packages = [
  {
    name: "basic",
    fileName: "instantclient-basic-windows.x64-19.31.0.0.0dbru.zip",
    url: "https://download.oracle.com/otn_software/nt/instantclient/1931000/instantclient-basic-windows.x64-19.31.0.0.0dbru.zip",
    sha256: "9e990e02e936fe073f05b7cb9bb95bd5163372083826e9321a9d85bff88e829d",
  },
  {
    name: "odbc",
    fileName: "instantclient-odbc-windows.x64-19.31.0.0.0dbru.zip",
    url: "https://download.oracle.com/otn_software/nt/instantclient/1931000/instantclient-odbc-windows.x64-19.31.0.0.0dbru.zip",
    sha256: "cdda3ba6bd47b7413eaf7b35b4fec903101a6e0623ce7a95724e8a3437c407d7",
  },
];
const requiredFiles = [
  "BASIC_LICENSE",
  "ODBC_LICENSE",
  "oci.dll",
  "oraociei19.dll",
  "sqora32.dll",
  "sqoras32.dll",
  "odbc_install.exe",
  "odbc_uninstall.exe",
];

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const cacheDirectory = path.join(
  repositoryRoot,
  ".cache",
  "oracle-instant-client",
  VERSION,
);
const resourceDirectory = path.join(
  repositoryRoot,
  "src-tauri",
  "resources",
  "oracle",
  BUNDLE_DIRECTORY,
);
const markerPath = path.join(resourceDirectory, "BUNDLE_MANIFEST.json");
const verifyOnly = process.argv.includes("--verify");

async function exists(filePath) {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function sha256(filePath) {
  const hash = createHash("sha256");
  await pipeline(createReadStream(filePath), hash);
  return hash.digest("hex");
}

async function downloadPackage(definition) {
  const archivePath = path.join(cacheDirectory, definition.fileName);
  if (
    (await exists(archivePath)) &&
    (await sha256(archivePath)) === definition.sha256
  ) {
    console.log(`Oracle ${definition.name} 已命中校验缓存`);
    return archivePath;
  }
  await rm(archivePath, { force: true });
  const response = await fetch(definition.url, { redirect: "follow" });
  if (!response.ok || !response.body) {
    throw new Error(
      `Oracle ${definition.name} 下载失败：HTTP ${response.status}`,
    );
  }
  const temporaryPath = `${archivePath}.download`;
  await rm(temporaryPath, { force: true });
  await pipeline(
    Readable.fromWeb(response.body),
    createWriteStream(temporaryPath),
  );
  const actualHash = await sha256(temporaryPath);
  if (actualHash !== definition.sha256) {
    await rm(temporaryPath, { force: true });
    throw new Error(
      `Oracle ${definition.name} SHA-256 不匹配：${actualHash}`,
    );
  }
  await rm(archivePath, { force: true });
  await cp(temporaryPath, archivePath);
  await rm(temporaryPath, { force: true });
  return archivePath;
}

function extractArchive(archivePath, destination) {
  const result = spawnSync(
    "tar.exe",
    ["-xf", archivePath, "-C", destination],
    { stdio: "inherit" },
  );
  if (result.status !== 0) {
    throw new Error(`无法解压 Oracle 驱动包：${path.basename(archivePath)}`);
  }
}

async function mergeDirectory(source, destination) {
  for (const entry of await readdir(source, { withFileTypes: true })) {
    await cp(path.join(source, entry.name), path.join(destination, entry.name), {
      recursive: true,
      force: true,
    });
  }
}

async function verifyPreparedDriver() {
  for (const fileName of requiredFiles) {
    if (!(await exists(path.join(resourceDirectory, fileName)))) {
      throw new Error(`Oracle 内置驱动缺少文件：${fileName}`);
    }
  }
  const marker = JSON.parse(await readFile(markerPath, "utf8"));
  if (
    marker.version !== VERSION ||
    marker.driverName !== `Oracle in ${BUNDLE_DIRECTORY}`
  ) {
    throw new Error("Oracle 内置驱动版本或专用驱动名不匹配");
  }
  for (const definition of packages) {
    if (marker.packages?.[definition.name]?.sha256 !== definition.sha256) {
      throw new Error(`Oracle ${definition.name} 清单校验值不匹配`);
    }
  }
  console.log(
    `Oracle Instant Client ${VERSION} Windows x64 内置驱动已就绪：${resourceDirectory}`,
  );
}

async function prepare() {
  await mkdir(cacheDirectory, { recursive: true });
  const archivePaths = new Map();
  for (const definition of packages) {
    archivePaths.set(definition.name, await downloadPackage(definition));
  }

  const extractionRoot = path.join(cacheDirectory, "extracting");
  await rm(extractionRoot, { recursive: true, force: true });
  await mkdir(extractionRoot, { recursive: true });
  await rm(resourceDirectory, { recursive: true, force: true });
  await mkdir(resourceDirectory, { recursive: true });

  for (const definition of packages) {
    const destination = path.join(extractionRoot, definition.name);
    await mkdir(destination, { recursive: true });
    extractArchive(archivePaths.get(definition.name), destination);
    await mergeDirectory(
      path.join(destination, ARCHIVE_DIRECTORY),
      resourceDirectory,
    );
  }

  await writeFile(
    markerPath,
    `${JSON.stringify(
      {
        version: VERSION,
        platform: "windows-x64",
        driverName: `Oracle in ${BUNDLE_DIRECTORY}`,
        distributionLicense: "Oracle Free Use Terms and Conditions",
        officialDownloadPage:
          "https://www.oracle.com/database/technologies/instant-client/winx64-64-downloads.html",
        packages: Object.fromEntries(
          packages.map((definition) => [
            definition.name,
            { url: definition.url, sha256: definition.sha256 },
          ]),
        ),
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
  await rm(extractionRoot, { recursive: true, force: true });
  await verifyPreparedDriver();
}

if (process.platform !== "win32") {
  console.log("Oracle Windows x64 驱动准备仅在 Windows 构建中执行");
} else if (verifyOnly) {
  await verifyPreparedDriver();
} else {
  await prepare();
}
