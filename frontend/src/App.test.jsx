import React from "react";
import {
  act,
  render,
  screen,
  waitFor,
  fireEvent,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi, it, expect, beforeEach } from "vitest";
import App from "./App";
const plan = {
  id: "p1",
  name: "Home backup",
  enabled: true,
  schedule: "0 0 2 * * *",
  timezone: "UTC",
  sources: [{ name: "data", path: "/data" }],
  remotes: [{ name: "local", directory: "/backup" }],
  retry: {
    max_attempts: 3,
    initial_delay_seconds: 10,
    max_delay_seconds: 300,
    backoff: "exponential",
  },
  archive: { kind: "7z", password: "••••••••", suffix: "%Y%m%d" },
  retention: { keep_days: 30, keep_count: 5 },
  rclone_flags: [],
  upload_concurrency: 1,
  remote_check_concurrency: 4,
};
const responses = {
  "/api/status": {
    rclone_ready: true,
    rclone_quarantined: false,
    authentication_enabled: true,
  },
  "/api/health": { version: "2.3.1", site_name: "Rclone Backup" },
  "/api/plans": [plan],
  "/api/runs?include_logs=false": [],
  "/api/rclone/remotes": { remotes: [{ name: "local", type: "local" }] },
  "/api/notifications": { config: { targets: [], templates: [] } },
  "/api/metrics": { successful_runs_24h: 55 },
};
beforeEach(() => {
  localStorage.setItem("language", "en");
  localStorage.setItem("world", "minimal");
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path) => ({
      ok: true,
      status: 200,
      json: async () => structuredClone(responses[path] ?? {}),
    })),
  );
});
it("uses the complete 24-hour count and renders real plan data", async () => {
  render(<App />);
  await screen.findByText("Home backup");
  expect(screen.getByText("55")).toBeTruthy();
  expect(screen.getByText("v2.3.1")).toBeTruthy();
});
it("switches and persists all four styles, language and theme", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Home backup");
  for (const world of ["toy", "craft", "retro", "minimal"]) {
    await user.selectOptions(screen.getByLabelText("Interface style"), world);
    expect(document.documentElement.dataset.world).toBe(world);
    expect(localStorage.getItem("world")).toBe(world);
  }
  await user.click(screen.getByRole("button", { name: "Switch language" }));
  expect(
    screen.getByRole("heading", { name: "备份方案", level: 1 }),
  ).toBeTruthy();
  expect(localStorage.getItem("language")).toBe("zh");
  await user.click(screen.getByRole("button", { name: /切换主题/ }));
  expect(localStorage.getItem("theme")).toBe("light");
});
it("supports deep routes and browser history", async () => {
  history.replaceState(null, "", "/history");
  render(<App />);
  await screen.findByText("No runs yet");
  fireEvent.click(screen.getByRole("link", { name: /Storage accounts/ }));
  await screen.findByRole("heading", { name: "Storage accounts", level: 1 });
  expect(location.pathname).toBe("/accounts");
  history.replaceState(null, "", "/plans");
  window.dispatchEvent(new PopStateEvent("popstate"));
  await screen.findByText("Home backup");
});
it("retains the archive password sentinel when saving an edited plan", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Home backup");
  await user.click(screen.getByRole("button", { name: "Edit Home backup" }));
  await user.click(screen.getByRole("button", { name: "Save plan" }));
  await waitFor(() =>
    expect(
      fetch.mock.calls.some(
        ([url, opts]) => url === "/api/plans/p1" && opts?.method === "PUT",
      ),
    ).toBe(true),
  );
  const [, options] = fetch.mock.calls.find(
    ([url, opts]) => url === "/api/plans/p1" && opts?.method === "PUT",
  );
  expect(JSON.parse(options.body).archive.password).toBe("••••••••");
});
it("a cancelled delete does not send a mutation", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Home backup");
  await user.click(screen.getByRole("button", { name: "Delete Home backup" }));
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  expect(fetch.mock.calls.some(([, opts]) => opts?.method === "DELETE")).toBe(
    false,
  );
});
it("keeps notification drafts while background data refreshes", async () => {
  const user = userEvent.setup();
  history.replaceState(null, "", "/notifications");
  render(<App />);
  await screen.findByText("No notification targets");
  await user.click(screen.getByRole("button", { name: "Add target" }));
  await user.clear(screen.getByLabelText("Target name"));
  await user.type(screen.getByLabelText("Target name"), "Draft webhook");
  await user.click(
    screen.getByRole("link", { name: "Notification templates" }),
  );
  await user.click(screen.getByRole("link", { name: "Notifications" }));
  expect(screen.getByLabelText("Target name").value).toBe("Draft webhook");
  expect(screen.getByText("You have unsaved changes")).toBeTruthy();
});
it("clears archive retention when changing to native-directory backup", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Home backup");
  await user.click(screen.getByRole("button", { name: "Edit Home backup" }));
  await user.selectOptions(screen.getByLabelText("Archive format"), "none");
  await user.click(screen.getByRole("button", { name: "Save plan" }));
  await waitFor(() =>
    expect(
      fetch.mock.calls.some(
        ([url, options]) =>
          url === "/api/plans/p1" && options?.method === "PUT",
      ),
    ).toBe(true),
  );
  const [, options] = fetch.mock.calls.find(
    ([url, options]) => url === "/api/plans/p1" && options?.method === "PUT",
  );
  expect(JSON.parse(options.body).retention).toEqual({
    keep_days: 0,
    keep_count: 0,
  });
});

it("keeps schema-unknown remote values editable and secrets masked", async () => {
  const user = userEvent.setup();
  const fetchDefault = fetch.getMockImplementation();
  fetch.mockImplementation(async (path, options) => {
    if (path === "/api/rclone/remotes")
      return {
        ok: true,
        status: 200,
        json: async () => ({
          remotes: [
            {
              name: "custom",
              type: "local",
              parameters: { legacy_option: "kept" },
              configured_secrets: ["headers"],
            },
          ],
        }),
      };
    if (path === "/api/rclone/providers")
      return {
        ok: true,
        status: 200,
        json: async () => ({ providers: [{ Name: "local", Options: [] }] }),
      };
    return fetchDefault(path, options);
  });
  history.replaceState(null, "", "/accounts");
  render(<App />);
  await screen.findByText("custom");
  await user.click(screen.getByRole("button", { name: "Edit custom" }));
  expect((await screen.findByLabelText("legacy_option")).value).toBe("kept");
  expect(screen.getByLabelText("headers").type).toBe("password");
});

it("returns focus to the opener after closing a plan dialog with Escape", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Home backup");
  const opener = screen.getByRole("button", { name: "Edit Home backup" });
  await user.click(opener);
  await screen.findByRole("dialog");
  await user.keyboard("{Escape}");
  await waitFor(() => expect(document.activeElement).toBe(opener));
});
it("returns focus to the delete button after cancelling confirmation", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("Home backup");
  const opener = screen.getByRole("button", { name: "Delete Home backup" });
  await user.click(opener);
  await user.click(screen.getByRole("button", { name: "Cancel" }));
  await waitFor(() => expect(document.activeElement).toBe(opener));
});
