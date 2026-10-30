import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { useRunDetails } from "./History";
it("ignores an old detail response after the selected run changes", async () => {
  const pending = new Map();
  vi.stubGlobal(
    "fetch",
    vi.fn((path) => new Promise((resolve) => pending.set(path, resolve))),
  );
  const { result, rerender } = renderHook(({ id }) => useRunDetails(id), {
    initialProps: { id: "old" },
  });
  rerender({ id: "new" });
  await act(async () => {
    pending.get("/api/runs/new")({
      ok: true,
      status: 200,
      json: async () => ({ id: "new", log: "latest" }),
    });
  });
  await waitFor(() => expect(result.current.run?.log).toBe("latest"));
  await act(async () => {
    pending.get("/api/runs/old")({
      ok: true,
      status: 200,
      json: async () => ({ id: "old", log: "stale" }),
    });
  });
  expect(result.current.run.id).toBe("new");
});
it("does not start another detail request until the current one finishes", async () => {
  vi.useFakeTimers();
  try {
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise(() => {})),
    );
    const { unmount } = renderHook(() => useRunDetails("running"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(fetch).toHaveBeenCalledTimes(1);
    unmount();
  } finally {
    vi.useRealTimers();
  }
});
