const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

function loadFunction(name) {
  const source = fs.readFileSync(new URL("app.js", `file://${__dirname}/`), "utf8");
  const start = source.indexOf(`function ${name}(`);
  assert.notEqual(start, -1, `${name} must exist in app.js`);
  const bodyStart = source.indexOf("{", start);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") depth -= 1;
    if (depth === 0) return vm.runInNewContext(`(${source.slice(start, index + 1)})`);
  }
  throw new Error(`Could not parse ${name}`);
}

test("run details normalize parsed progress before rendering", () => {
  const source = fs.readFileSync(new URL("app.js", `file://${__dirname}/`), "utf8");
  assert.match(source, /const progress = normalizeRunProgress\(run, parseRunProgress\(run\)\);/);
});

test("terminal interruption replaces stale uploading progress", () => {
  const normalizeRunProgress = loadFunction("normalizeRunProgress");
  const finishedAt = "2026-08-18T10:05:00Z";
  const progress = {
    phase: "uploading",
    updatedAt: "2026-08-18T10:04:00Z",
    targets: new Map([
      ["done", { name: "done", status: "success", at: "2026-08-18T10:03:00Z" }],
      ["stale", { name: "stale", status: "uploading", startedAt: "2026-08-18T10:02:00Z" }],
    ]),
  };

  const normalized = normalizeRunProgress({ status: "interrupted", finished_at: finishedAt }, progress);

  assert.equal(normalized.phase, "interrupted");
  assert.equal(normalized.updatedAt, finishedAt);
  assert.equal(normalized.targets.get("done").status, "success");
  assert.deepEqual(
    { ...normalized.targets.get("stale") },
    { name: "stale", status: "interrupted", startedAt: "2026-08-18T10:02:00Z", at: finishedAt },
  );
});

test("active progress remains live", () => {
  const normalizeRunProgress = loadFunction("normalizeRunProgress");
  const progress = {
    phase: "uploading",
    updatedAt: "2026-08-18T10:04:00Z",
    targets: new Map([["live", { name: "live", status: "uploading" }]]),
  };

  const normalized = normalizeRunProgress({ status: "running", finished_at: null }, progress);

  assert.equal(normalized.phase, "uploading");
  assert.equal(normalized.targets.get("live").status, "uploading");
});
