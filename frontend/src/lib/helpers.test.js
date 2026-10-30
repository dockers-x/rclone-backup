import { describe, it, expect } from "vitest";
import {
  parseRunProgress,
  normalizeRunProgress,
  parseSimpleSchedule,
  splitArgs,
  joinArgs,
  renderTemplateValue,
  providerFieldIsSecret,
} from "./helpers";
import { scheduleCron } from "../pages/PlanModal";
describe("backup data compatibility", () => {
  it("normalizes interrupted uploads while preserving finished targets", () => {
    const progress = {
      phase: "uploading",
      targets: new Map([
        ["done", { status: "success" }],
        ["live", { status: "uploading", startedAt: "start" }],
      ]),
    };
    const result = normalizeRunProgress(
      { status: "interrupted", finished_at: "end" },
      progress,
    );
    expect(result.phase).toBe("interrupted");
    expect(result.targets.get("live")).toEqual({
      status: "interrupted",
      startedAt: "start",
      at: "end",
    });
    expect(result.targets.get("done").status).toBe("success");
  });
  it("does not replace live progress", () => {
    const progress = { phase: "uploading", targets: new Map() };
    expect(normalizeRunProgress({ status: "running" }, progress)).toBe(
      progress,
    );
  });
  it("parses target events and keeps unstructured logs", () => {
    const result = parseRunProgress({
      status: "running",
      log: 'Starting\n@event {"kind":"target","name":"cloud","directory":"/backup","status":"uploading","at":"2026-10-30T00:00:00Z"}\n@event invalid',
    });
    expect([...result.targets.values()][0].startedAt).toBe(
      "2026-10-30T00:00:00Z",
    );
    expect(result.log).toEqual(["Starting", "@event invalid"]);
  });
  it.each([
    { kind: "daily", time: "02:30" },
    { kind: "weekly", time: "23:15", weekday: "FRI" },
    { kind: "monthly", time: "08:05", monthday: "28" },
    { kind: "every_seconds", interval: "15" },
    { kind: "every_minutes", interval: "10" },
    { kind: "every_hours", interval: "6" },
  ])("round trips simple schedule $kind", (schedule) => {
    expect(parseSimpleSchedule(scheduleCron(schedule))).toEqual(schedule);
  });
  it("retains unsupported cron as an advanced expression", () => {
    expect(parseSimpleSchedule("0 0 0 1 JAN * 2027")).toBeNull();
  });
  it("handles quoted rclone flag values", () => {
    const args = ["--exclude", "my folder/**", "--transfers", "4"];
    expect(splitArgs(joinArgs(args))).toEqual(args);
  });
  it("previews Chinese and English templates", () => {
    expect(
      renderTemplateValue("{{event}} / {{backup_size}}", "success", "zh"),
    ).toBe("成功 / 1.50 GiB");
    expect(
      renderTemplateValue("{{event}} / {{backup_size}}", "start", "en"),
    ).toBe("start / Not available");
  });
});

it.each([
  "headers",
  "key",
  "credentials",
  "connection_string",
  "cookies",
  "key_pem",
  "sas_url",
  "pass",
])("masks provider secret %s", (name) => {
  expect(providerFieldIsSecret({}, name)).toBe(true);
});
it("honors Sensitive metadata without hiding public identifiers", () => {
  expect(providerFieldIsSecret({ Sensitive: true }, "custom_field")).toBe(true);
  expect(providerFieldIsSecret({ Sensitive: true }, "client_id")).toBe(false);
});
