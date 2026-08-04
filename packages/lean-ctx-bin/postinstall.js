#!/usr/bin/env node
"use strict";

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { createGunzip } = require("zlib");
const crypto = require("crypto");

const REPO = "yvgude/lean-ctx";
const BIN_DIR = path.join(__dirname, "bin");
const IS_WIN = process.platform === "win32";
const BINARY_NAME = IS_WIN ? "lean-ctx.exe" : "lean-ctx";
const BINARY_PATH = path.join(BIN_DIR, BINARY_NAME);
const DOWNLOAD_TIMEOUT_MS = 60000;
const API_TIMEOUT_MS = 15000;

function getGlibcVersion() {
  try {
    const out = execSync("ldd --version 2>&1 || true", { encoding: "utf8" });
    const match = out.match(/(\d+)\.(\d+)\s*$/m);
    if (match) return { major: parseInt(match[1]), minor: parseInt(match[2]) };
  } catch {}
  return null;
}

function getTarget() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "linux") {
    let libc = "musl";
    const glibc = getGlibcVersion();
    if (glibc && (glibc.major > 2 || (glibc.major === 2 && glibc.minor >= 35))) {
      libc = "gnu";
    }
    const archMap = { x64: "x86_64", arm64: "aarch64" };
    const rustArch = archMap[arch];
    if (!rustArch) {
      console.error(`Unsupported architecture: ${arch}`);
      process.exit(1);
    }
    return `${rustArch}-unknown-linux-${libc}`;
  }

  const key = `${platform}-${arch}`;
  const targets = {
    "darwin-x64": "x86_64-apple-darwin",
    "darwin-arm64": "aarch64-apple-darwin",
    "win32-x64": "x86_64-pc-windows-msvc",
  };

  const target = targets[key];
  if (!target) {
    console.error(`Unsupported platform: ${key}`);
    console.error("Build from source instead: cargo install lean-ctx");
    process.exit(1);
  }
  return target;
}

function httpsGet(url, timeoutMs = API_TIMEOUT_MS) {
  return new Promise((resolve, reject) => {
    const get = (u, redirects = 0) => {
      if (redirects > 5) {
        reject(new Error(`Too many redirects for ${url}`));
        return;
      }
      const req = https.get(u, { headers: { "User-Agent": "lean-ctx-bin-npm" } }, (res) => {
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          get(res.headers.location, redirects + 1);
          return;
        }
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          return;
        }
        resolve(res);
      });
      req.on("error", reject);
      req.setTimeout(timeoutMs, () => {
        req.destroy();
        reject(new Error(
          `Request timed out after ${timeoutMs / 1000}s for ${u}\n` +
          `  This usually means a firewall, proxy, or antivirus is blocking the connection.\n` +
          `  Workaround: download manually from https://github.com/${REPO}/releases/latest`
        ));
      });
    };
    get(url);
  });
}

function httpsGetJson(url) {
  return new Promise((resolve, reject) => {
    httpsGet(url, API_TIMEOUT_MS).then((res) => {
      let data = "";
      res.on("data", (c) => (data += c));
      res.on("end", () => {
        try { resolve(JSON.parse(data)); }
        catch (e) { reject(e); }
      });
    }).catch(reject);
  });
}

async function downloadToFile(url, dest) {
  console.log("lean-ctx: downloading binary...");
  const res = await httpsGet(url, DOWNLOAD_TIMEOUT_MS);
  const total = parseInt(res.headers["content-length"] || "0", 10);
  let received = 0;
  let lastPct = -1;

  return new Promise((resolve, reject) => {
    const ws = fs.createWriteStream(dest);
    res.on("data", (chunk) => {
      received += chunk.length;
      if (total > 0) {
        const pct = Math.floor((received / total) * 100);
        if (pct >= lastPct + 10) {
          lastPct = pct;
          process.stdout.write(`\r  ${pct}% (${(received / 1024 / 1024).toFixed(1)} MB)`);
        }
      }
    });
    res.pipe(ws);
    ws.on("finish", () => {
      if (total > 0) process.stdout.write(`\r  100% (${(received / 1024 / 1024).toFixed(1)} MB)\n`);
      resolve();
    });
    ws.on("error", reject);
  });
}

function extractTarGz(archive, destDir, binaryName) {
  const gunzip = createGunzip();
  const input = fs.createReadStream(archive);

  return new Promise((resolve, reject) => {
    const chunks = [];
    input.pipe(gunzip)
      .on("data", (c) => chunks.push(c))
      .on("end", () => {
        const buf = Buffer.concat(chunks);
        let offset = 0;
        while (offset < buf.length) {
          const header = buf.subarray(offset, offset + 512);
          if (header.every((b) => b === 0)) break;

          const name = header.subarray(0, 100).toString("utf8").replace(/\0/g, "");
          const sizeStr = header.subarray(124, 136).toString("utf8").replace(/\0/g, "").trim();
          const size = parseInt(sizeStr, 8) || 0;
          offset += 512;

          const baseName = path.basename(name);
          if (baseName === binaryName && size > 0) {
            const dest = path.join(destDir, binaryName);
            fs.writeFileSync(dest, buf.subarray(offset, offset + size));
            if (!IS_WIN) fs.chmodSync(dest, 0o755);
            resolve(dest);
            return;
          }
          offset += Math.ceil(size / 512) * 512;
        }
        reject(new Error(`${binaryName} not found in archive`));
      })
      .on("error", reject);
  });
}

function runOnboard(binaryPath) {
  if (process.env.CI || process.env.LEAN_CTX_NO_ONBOARD === "1") return;
  try {
    console.log("");
    console.log("Running onboard (connecting your AI tools)...");
    execSync(`"${binaryPath}" onboard`, { stdio: "inherit", timeout: 30000 });
  } catch {
    // Non-fatal: onboard may fail in restricted envs
  }
}

function printSuccess() {
  console.log("");
  console.log("\x1b[1m\u250c\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2510\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m  \x1b[32m\x1b[1m\u2713 lean-ctx installed successfully\x1b[0m                  \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m                                                     \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m  Quick start:                                       \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m    \x1b[1mlean-ctx wrap cursor\x1b[0m  (one-command setup)        \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m    \x1b[1mlean-ctx wrap claude\x1b[0m  (Claude Code)              \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m    \x1b[1mlean-ctx wrap codex\x1b[0m   (Codex CLI)                \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m                                                     \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m  Full control: \x1b[2mlean-ctx setup\x1b[0m                        \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2502\x1b[0m  \x1b[2mDocs: https://leanctx.com/docs\x1b[0m                     \x1b[1m\u2502\x1b[0m");
  console.log("\x1b[1m\u2514\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2518\x1b[0m");
}

async function main() {
  if (fs.existsSync(BINARY_PATH)) {
    console.log("lean-ctx binary already exists, skipping download");
    runOnboard(BINARY_PATH);
    printSuccess();
    return;
  }

  const target = getTarget();
  console.log(`lean-ctx: installing for ${target}...`);

  console.log("lean-ctx: fetching release info from GitHub...");
  const release = await httpsGetJson(`https://api.github.com/repos/${REPO}/releases/latest`);
  const tag = release.tag_name;
  console.log(`lean-ctx: latest release ${tag}`);

  const ext = IS_WIN ? ".zip" : ".tar.gz";
  const assetName = `lean-ctx-${target}${ext}`;
  const asset = (release.assets || []).find((a) => a.name === assetName);
  if (!asset) {
    console.error(`No binary for ${target}. Install from source: cargo install lean-ctx`);
    process.exit(1);
  }

  const tmpDir = fs.mkdtempSync(path.join(require("os").tmpdir(), "lean-ctx-"));
  const archivePath = path.join(tmpDir, assetName);

  try {
    await downloadToFile(asset.browser_download_url, archivePath);

    const sumsAsset = (release.assets || []).find((a) => a.name === "SHA256SUMS");
    if (sumsAsset) {
      const sumsText = await new Promise((resolve, reject) => {
        httpsGet(sumsAsset.browser_download_url).then((res) => {
          let data = "";
          res.on("data", (c) => (data += c));
          res.on("end", () => resolve(data));
        }).catch(reject);
      });
      const expectedLine = sumsText.split("\n").find((l) => l.includes(assetName));
      if (expectedLine) {
        const expectedHash = expectedLine.trim().split(/\s+/)[0].toLowerCase();
        const fileHash = crypto.createHash("sha256").update(fs.readFileSync(archivePath)).digest("hex");
        if (fileHash !== expectedHash) {
          throw new Error(`SHA256 mismatch: expected ${expectedHash}, got ${fileHash}. Binary may be compromised.`);
        }
        console.log("lean-ctx: SHA256 verified");
      }
    }

    console.log("lean-ctx: extracting...");

    fs.mkdirSync(BIN_DIR, { recursive: true });

    if (IS_WIN) {
      const oldBin = BINARY_PATH + ".old";
      try { fs.unlinkSync(oldBin); } catch {}
      try { fs.renameSync(BINARY_PATH, oldBin); } catch {}
      execSync(`tar -xf "${archivePath}" -C "${BIN_DIR}"`, { stdio: "ignore" });
      try { fs.unlinkSync(oldBin); } catch {}
    } else {
      await extractTarGz(archivePath, BIN_DIR, "lean-ctx");
    }

    console.log(`lean-ctx: installed to ${BINARY_PATH}`);
    runOnboard(BINARY_PATH);
    printSuccess();
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}

main().catch((err) => {
  console.error(`\nlean-ctx: installation failed: ${err.message}`);
  console.error("");
  console.error("Manual install options:");
  console.error("  1. Download from https://github.com/yvgude/lean-ctx/releases/latest");
  console.error("  2. cargo install lean-ctx");
  console.error("  3. brew install yvgude/tap/lean-ctx (macOS/Linux)");
  process.exit(1);
});
