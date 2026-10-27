const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const vm = require("node:vm");

function loadFunction(name, context = {}) {
  const source = fs.readFileSync(new URL("app.js", `file://${__dirname}/`), "utf8");
  const start = source.indexOf(`function ${name}(`);
  assert.notEqual(start, -1, `${name} must exist in app.js`);
  const declarationStart = source.slice(start - 6, start) === "async " ? start - 6 : start;
  const bodyStart = source.indexOf("{", start);
  let depth = 0;
  for (let index = bodyStart; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") depth -= 1;
    if (depth === 0) return vm.runInNewContext(`(${source.slice(declarationStart, index + 1)})`, context);
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


test("dashboard uses the full-history count instead of the 50 visible runs", () => {
  const elements = new Map();
  const state = {
    plans: [{ enabled: true }],
    runs: Array.from({ length: 50 }, () => ({ status: "success" })),
    metrics: { successful_runs_24h: 55 },
    remotes: [],
  };
  const renderMetrics = loadFunction("renderMetrics", {
    state,
    $: (selector) => {
      if (!elements.has(selector)) elements.set(selector, {});
      return elements.get(selector);
    },
  });
  renderMetrics();
  assert.equal(elements.get("#successCount").textContent, 55);
});

test("a late run-detail response cannot overwrite newer progress", async () => {
  const responses = [];
  const state = { openRunId: "run-1", openRun: null };
  let renders = 0;
  const refreshOpenRun = loadFunction("refreshOpenRun", {
    state,
    runDetailsRequest: 0,
    $: () => ({ open: true }),
    api: () => new Promise((resolve) => responses.push(resolve)),
    renderOpenRun: () => { renders += 1; },
  });
  const older = refreshOpenRun();
  const newer = refreshOpenRun();
  responses[1]({ id: "run-1", log: "new" });
  await newer;
  responses[0]({ id: "run-1", log: "old" });
  await older;
  assert.equal(state.openRun.log, "new");
  assert.equal(renders, 1);
});

test("closing a run dialog discards an outstanding detail response", async () => {
  let resolveResponse;
  const state = { openRunId: "run-1", openRun: null };
  const dialog = { open: true };
  const refreshOpenRun = loadFunction("refreshOpenRun", {
    state,
    runDetailsRequest: 0,
    $: () => dialog,
    api: () => new Promise((resolve) => { resolveResponse = resolve; }),
    renderOpenRun: () => assert.fail("a closed dialog must not be rendered"),
  });
  const request = refreshOpenRun();
  dialog.open = false;
  resolveResponse({ id: "run-1", log: "content" });
  await request;
  assert.equal(state.openRun, null);
});
