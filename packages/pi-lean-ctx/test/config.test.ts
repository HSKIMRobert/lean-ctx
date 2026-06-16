import { afterEach, describe, expect, it } from "vitest";

import {
  REPLACEABLE_BUILTIN_TOOLS,
  resolveRouteShell,
  resolveSuppressedBuiltins,
} from "../extensions/config.js";

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

describe("resolveSuppressedBuiltins", () => {
  it("replace mode suppresses all five natives (only ctx_* remain)", () => {
    const suppressed = resolveSuppressedBuiltins("replace", true);
    expect([...suppressed].sort()).toEqual(["bash", "find", "grep", "ls", "read"]);
  });

  it("additive + routeShell suppresses only native bash (the R1 102-bash/0-ctx_shell guard)", () => {
    const suppressed = resolveSuppressedBuiltins("additive", true);
    expect([...suppressed]).toEqual(["bash"]);
    // read/ls/find/grep stay available next to their ctx_* counterparts.
    expect(suppressed.has("read")).toBe(false);
  });

  it("additive without routeShell suppresses nothing (non-regressive default)", () => {
    expect(resolveSuppressedBuiltins("additive", false).size).toBe(0);
  });

  it("any faithful arm (replace or routeShell) removes native bash so shell must route through ctx_shell", () => {
    expect(resolveSuppressedBuiltins("replace", true).has("bash")).toBe(true);
    expect(resolveSuppressedBuiltins("additive", true).has("bash")).toBe(true);
  });

  it("never suppresses a builtin without shipping a ctx_* replacement", () => {
    const replaceable = new Set<string>(REPLACEABLE_BUILTIN_TOOLS);
    for (const mode of ["additive", "replace"] as const) {
      for (const routeShell of [false, true]) {
        for (const name of resolveSuppressedBuiltins(mode, routeShell)) {
          expect(replaceable.has(name)).toBe(true);
        }
      }
    }
  });
});
