document.addEventListener("DOMContentLoaded", () => {
  const tokensSaved = document.getElementById("tokens-saved");
  const commands = document.getElementById("commands");
  const toggleEnabled = document.getElementById("toggle-enabled");
  const toggleNative = document.getElementById("toggle-native");

  chrome.runtime.sendMessage({ action: "getStats" }, (stats) => {
    if (stats) {
      tokensSaved.textContent = formatNumber(stats.totalSaved || 0);
      commands.textContent = String(stats.totalCommands || 0);
    }
  });

  chrome.runtime.sendMessage({ action: "getSettings" }, (settings) => {
    if (settings) {
      toggleEnabled.checked = settings.enabled !== false;
      toggleNative.checked = settings.useNative !== false;
    }
  });

  toggleEnabled.addEventListener("change", () => {
    updateSetting("enabled", toggleEnabled.checked);
  });

  toggleNative.addEventListener("change", () => {
    updateSetting("useNative", toggleNative.checked);
  });

  checkNativeStatus();
});

function updateSetting(key, value) {
  chrome.storage.local.get(["settings"], (result) => {
    const settings = result.settings || {};
    settings[key] = value;
    chrome.storage.local.set({ settings });
  });
}

function formatNumber(n) {
  if (n >= 1000000) return (n / 1000000).toFixed(1) + "M";
  if (n >= 1000) return (n / 1000).toFixed(1) + "k";
  return String(n);
}

function checkNativeStatus() {
  const footer = document.querySelector(".footer");

  try {
    chrome.runtime.sendMessage({ action: "pingNative" }, (response) => {
      if (response && response.nativeOk) {
        footer.innerHTML = `
          <span style="color:#00d4aa">Native messaging active</span><br>
          <a href="https://leanctx.com" target="_blank">leanctx.com</a> ·
          <a href="https://github.com/yvgude/lean-ctx" target="_blank">GitHub</a>
        `;
      } else {
        showSetupHint(footer);
      }
    });
  } catch {
    showSetupHint(footer);
  }
}

function showSetupHint(footer) {
  const extId = chrome.runtime.id;
  const cmd = `cd ~/Documents/Privat/Projects/lean-ctx/packages/chrome-lean-ctx/native-host && chmod +x install.sh bridge.sh && ./install.sh ${extId}`;

  footer.innerHTML = `
    <div style="text-align:left;font-size:11px;margin-bottom:8px;color:#ff9800">
      Native messaging not connected
    </div>
    <div style="text-align:left;font-size:10px;color:#888;margin-bottom:6px">
      Run this in Terminal for full compression:
    </div>
    <div style="background:#1a1a2e;border-radius:6px;padding:8px;font-family:monospace;font-size:9px;
                word-break:break-all;cursor:pointer;border:1px solid #333;position:relative"
         id="copy-cmd" title="Click to copy">
      ${cmd}
    </div>
    <div id="copy-feedback" style="font-size:10px;color:#00d4aa;margin-top:4px;display:none">
      Copied to clipboard!
    </div>
    <div style="margin-top:8px">
      <a href="https://leanctx.com" target="_blank">leanctx.com</a> ·
      <a href="https://github.com/yvgude/lean-ctx" target="_blank">GitHub</a>
    </div>
  `;

  document.getElementById("copy-cmd").addEventListener("click", () => {
    navigator.clipboard.writeText(cmd).then(() => {
      const fb = document.getElementById("copy-feedback");
      fb.style.display = "block";
      setTimeout(() => fb.style.display = "none", 2000);
    });
  });
}
