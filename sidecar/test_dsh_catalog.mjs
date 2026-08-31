import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const child = spawnSync(process.execPath, [join(here, "dsh_catalog.mjs")], {
  env: {
    PATH: process.env.PATH,
    BATUTA_DSH_CATALOG_MODULE: pathToFileURL(join(here, "fake_dsh_catalog.mjs")).href,
  },
  input: `${JSON.stringify({ schema_version: 1, id: "test-1", method: "catalog_snapshot" })}\n`,
  encoding: "utf8",
});
const stdout = child.stdout;
const stderr = child.stderr;
const code = child.status;

assert.equal(code, 0, `${stderr}\n${stdout}`);
const lines = stdout.trim().split("\n");
assert.equal(lines.length, 1);
const response = JSON.parse(lines[0]);
assert.deepEqual(Object.keys(response).sort(), ["id", "ok", "result", "schema_version"]);
assert.equal(response.ok, true);
assert.equal(response.result.routes.length, 2);
assert.equal(response.result.routes[0].cost.input, null);
assert.equal(stdout.includes("must-never-escape"), false);
assert.equal(stdout.includes("balance"), false);
assert.equal(stdout.includes("stream"), false);
