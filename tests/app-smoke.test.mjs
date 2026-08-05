import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { transformWithEsbuild } from "vite";

test("main React flow compiles as JSX", async () => {
  const source = await readFile(new URL("../src/App.jsx", import.meta.url), "utf8");
  const result = await transformWithEsbuild(source, "App.jsx", {
    loader: "jsx",
    jsx: "automatic",
  });
  assert.match(result.code, /function App\(/);
});
