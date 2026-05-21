import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import { resolveBinaryPath } from "./binary";

interface McpConfig {
  servers?: Record<string, { type?: string; command: string; args?: string[] }>;
}

function getVscodeMcpPath(): string | null {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    return null;
  }
  return path.join(workspaceRoot, ".vscode", "mcp.json");
}

function getLegacyMcpPath(): string | null {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  if (!workspaceRoot) {
    return null;
  }
  return path.join(workspaceRoot, ".github", "copilot", "mcp.json");
}

function getCursorMcpPath(): string {
  return path.join(process.env.HOME ?? "", ".cursor", "mcp.json");
}

export function isMcpConfigured(): boolean {
  const paths = [
    getVscodeMcpPath(),
    getLegacyMcpPath(),
    getCursorMcpPath(),
  ].filter(Boolean);

  for (const configPath of paths) {
    if (!configPath || !fs.existsSync(configPath)) {
      continue;
    }
    try {
      const content = fs.readFileSync(configPath, "utf-8");
      const config: McpConfig = JSON.parse(content);
      if (config.servers?.["lean-ctx"]) {
        return true;
      }
    } catch {
      continue;
    }
  }
  return false;
}

export async function configureMcp(): Promise<void> {
  const binary = resolveBinaryPath();
  if (!binary) {
    vscode.window.showErrorMessage(
      "lean-ctx binary not found. Install: cargo install lean-ctx"
    );
    return;
  }

  const configPath = getVscodeMcpPath();
  if (!configPath) {
    vscode.window.showErrorMessage("No workspace folder open.");
    return;
  }

  const dir = path.dirname(configPath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }

  let config: McpConfig = { servers: {} };
  try {
    config = JSON.parse(fs.readFileSync(configPath, "utf-8"));
  } catch {
    // File doesn't exist or contains invalid JSON — start fresh
  }

  if (!config.servers) {
    config.servers = {};
  }

  config.servers["lean-ctx"] = {
    type: "stdio",
    command: binary,
  };

  fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + "\n");

  vscode.window.showInformationMessage(
    `lean-ctx MCP configured in ${path.relative(
      vscode.workspace.workspaceFolders![0].uri.fsPath,
      configPath
    )}`
  );
}

export async function offerMcpSetup(): Promise<void> {
  if (isMcpConfigured()) {
    return;
  }

  const binary = resolveBinaryPath();
  if (!binary) {
    return;
  }

  const action = await vscode.window.showInformationMessage(
    "lean-ctx detected but MCP not configured. Configure now?",
    "Configure",
    "Later"
  );

  if (action === "Configure") {
    await configureMcp();
  }
}
