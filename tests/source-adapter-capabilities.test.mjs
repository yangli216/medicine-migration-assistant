import test from "node:test";
import assert from "node:assert/strict";

import { sourceAdapterCapabilityAssessment } from "../src/sourceAdapterCapabilities.js";

const adapters = [
  {
    id: "GENERIC_DATABASE",
    name: "通用数据库查询",
    sourceModes: ["database"],
    databaseFamilies: ["oracle", "mysql"],
    migrationTasks: ["MEDICINE_BASE"],
    automaticDetection: false,
  },
  {
    id: "GENERIC_FILE",
    name: "通用文件导入",
    sourceModes: ["file"],
    databaseFamilies: [],
    migrationTasks: ["MEDICINE_BASE"],
    automaticDetection: false,
  },
  {
    id: "PHIS27",
    name: "二系列phis",
    sourceModes: ["database"],
    databaseFamilies: ["oracle"],
    migrationTasks: ["MEDICINE_BASE", "INVENTORY"],
    automaticDetection: true,
    sourceCompatibility: {
      product: "二系列phis",
      mode: "STRUCTURE_CONTRACT",
      declaredVersions: [],
      gate: "版本号仅作项目记录，以任务级来源表字段契约为准。",
    },
  },
];

test("药品接入能力区分专用适配、通用数据库和文件", () => {
  const result = sourceAdapterCapabilityAssessment(adapters, "MEDICINE_BASE");

  assert.equal(result.routes.length, 3);
  assert.match(result.routes[0].title, /二系列phis（Oracle）/);
  assert.match(result.routes[0].detail, /不代表 HIS 版本已认证/);
  assert.match(result.routes[0].compatibility[0], /按现场结构契约判定/);
  assert.match(result.routes[1].title, /2 类数据库/);
  assert.match(result.routes[2].title, /CSV \/ JSON/);
});

test("库存接入只展示真正声明库存能力的专用适配器", () => {
  const result = sourceAdapterCapabilityAssessment(adapters, "INVENTORY");

  assert.match(result.headline, /1 个专用适配器/);
  assert.match(result.routes[0].title, /二系列phis（Oracle）/);
  assert.match(result.routes[0].compatibility[0], /版本号仅作项目记录/);
  assert.match(result.routes[1].title, /不能仅因数据库可连接/);
  assert.doesNotMatch(result.routes[0].title, /通用数据库/);
});

test("没有库存适配器时不会将数据库连接能力误报为可迁移", () => {
  const result = sourceAdapterCapabilityAssessment(adapters.slice(0, 2), "INVENTORY");

  assert.match(result.headline, /没有可直接读取库存/);
  assert.equal(result.routes[0].tone, "warning");
  assert.match(result.routes[0].detail, /适配器开发与验收/);
});
