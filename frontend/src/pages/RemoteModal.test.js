import { expect, it } from "vitest";
import { providerValue } from "./RemoteModal";
it("normalizes rclone typed defaults without corrupting editable values", () => {
  expect(
    providerValue({ Type: "tristate" }, { Valid: true, Value: false }),
  ).toBe("false");
  expect(
    providerValue({ Type: "tristate" }, { Valid: false, Value: true }),
  ).toBe("");
  expect(providerValue({ Type: "spaceseplist" }, ["a", "b"])).toBe("a b");
  expect(providerValue({ Type: "commaseplist" }, ["a", "b"])).toBe("a,b");
  expect(providerValue({}, {})).toBe("");
});
