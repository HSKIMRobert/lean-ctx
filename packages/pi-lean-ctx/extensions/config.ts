import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { resolve } from "node:path";

/**
 * Shape of the optional Pi override file
 * `~/.pi/agent/extensions/pi-lean-ctx/config.json`.
 *
 * It lets users who only run lean-ctx through Pi keep every setting inside
 * their Pi configuration instead of juggling `LEAN_CTX_PI_*` environment
 * variables and `~/.lean-ctx/config.toml` (see issue #344). All fields are
 * optional; an absent or malformed file simply falls back to env vars and
 * built-in defaults.
 */
export interface PiLeanCtxFileConfig {
  /** Tool exposure: "additive" (Pi builtins + ctx_*) or "replace" (ctx_* only). */
  mode?: string;
  /**
   * Start the embedded MCP bridge (the persistent session cache). Default
   * `true`; set `false` (or `LEAN_CTX_PI_ENABLE_MCP=0`) to force the one-shot
   * CLI path, which cannot cache across calls.
   */
  enableMcp?: boolean;
  /** Absolute path to the lean-ctx binary (equivalent to `LEAN_CTX_BIN`). */
  binary?: string;
  /**
   * Extra environment forwarded to every lean-ctx subprocess. Use this to
   * override `~/.lean-ctx/config.toml` engine settings without touching that
   * file, since the engine honours `LEAN_CTX_*` env vars
   * (e.g. `{ "LEAN_CTX_COMPRESSION": "aggressive" }`).
   */
  env?: Record<string, string>;
}

export type PiMode = "additive" | "replace";

/** Fully resolved configuration after merging file, env vars and defaults. */
export interface ResolvedPiConfig {
  mode: PiMode;
  enableMcp: boolean;
  /** Binary path from the file; `LEAN_CTX_BIN` still takes precedence at use time. */
  binaryOverride?: string;
  /** Engine env overrides forwarded to lean-ctx subprocesses. */
  forwardedEnv: Record<string, string>;
  /** Absolute path the loader looked at (whether or not it existed). */
  configPath: string;
  /** True when the file existed and parsed into a JSON object. */
  loaded: boolean;
}

/** Absolute path to the Pi override file (Pi's per-extension config convention). */
export function piConfigPath(): string {
  return resolve(
    homedir(),
    ".pi",
    "agent",
    "extensions",
    "pi-lean-ctx",
    "config.json",
  );
}

function envFlag(name: string): boolean {
  const raw = process.env[name];
  if (!raw) return false;
  const v = raw.trim().toLowerCase();
  return v === "1" || v === "true" || v === "yes" || v === "on";
}

function readFileConfig(path: string): { cfg: PiLeanCtxFileConfig; loaded: boolean } {
  if (!existsSync(path)) return { cfg: {}, loaded: false };
  try {
    const parsed: unknown = JSON.parse(readFileSync(path, "utf8"));
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return { cfg: parsed as PiLeanCtxFileConfig, loaded: true };
    }
    console.error(`[pi-lean-ctx] ${path}: expected a JSON object — ignoring.`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error(`[pi-lean-ctx] ${path}: invalid JSON (${msg}) — ignoring.`);
  }
  return { cfg: {}, loaded: false };
}

function resolveMode(fileMode: string | undefined): PiMode {
  const raw = (process.env.LEAN_CTX_PI_MODE ?? fileMode ?? "additive").toLowerCase();
  return raw === "replace" ? "replace" : "additive";
}

/**
 * Loads and resolves the Pi override config. Precedence per setting is
 * "most explicit wins": an explicit `LEAN_CTX_PI_*` / `LEAN_CTX_BIN` env var
 * overrides `config.json`, which overrides the built-in default. This keeps
 * shareable, file-only setups working (no env vars needed) while still
 * allowing ad-hoc env overrides on a single machine.
 */
export function loadPiConfig(): ResolvedPiConfig {
  const configPath = piConfigPath();
  const { cfg, loaded } = readFileConfig(configPath);

  // The embedded MCP bridge holds the persistent session cache, so unchanged
  // re-reads cost ~13 tokens and reads register as CEP sessions. That is
  // lean-ctx's core value prop, so the bridge is ON by default; the one-shot CLI
  // path cannot cache across calls (#361). Opt out with LEAN_CTX_PI_ENABLE_MCP=0
  // or "enableMcp": false in config.json.
  const enableMcp =
    process.env.LEAN_CTX_PI_ENABLE_MCP !== undefined
      ? envFlag("LEAN_CTX_PI_ENABLE_MCP")
      : cfg.enableMcp !== false;

  const forwardedEnv: Record<string, string> = {};
  if (cfg.env && typeof cfg.env === "object" && !Array.isArray(cfg.env)) {
    for (const [key, value] of Object.entries(cfg.env)) {
      if (typeof value === "string") forwardedEnv[key] = value;
    }
  }

  const binaryOverride =
    typeof cfg.binary === "string" && cfg.binary.length > 0 ? cfg.binary : undefined;

  return {
    mode: resolveMode(cfg.mode),
    enableMcp,
    binaryOverride,
    forwardedEnv,
    configPath,
    loaded,
  };
}
