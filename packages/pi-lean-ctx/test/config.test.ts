import { afterEach, describe, expect, it } from "vitest";

import { resolveRouteShell } from "../extensions/config.js";

const ENV_KEY = "LEAN_CTX_PI_ROUTE_SHELL";

afterEach(() => {
  delete process.env[ENV_KEY];
});

describe("resolveRouteShell", () => {
  it("replace mode always routes shell (every builtin is suppressed anyway)", () => {
    expect(resolveRouteShell("replace", false)).toBe(true);
    expect(resolveRouteShell("replace", undefined)).toBe(true);
  });

  it("additive mode defaults off so native bash stays available (non-regressive)", () => {
    expect(resolveRouteShell("additive", undefined)).toBe(false);
    expect(resolveRouteShell("additive", false)).toBe(false);
  });

  it("additive mode honors the file flag when no env var is set", () => {
    expect(resolveRouteShell("additive", true)).toBe(true);
  });

  it("env var wins over the file flag in additive mode", () => {
    process.env[ENV_KEY] = "0";
    expect(resolveRouteShell("additive", true)).toBe(false);
    process.env[ENV_KEY] = "1";
    expect(resolveRouteShell("additive", false)).toBe(true);
  });
});
