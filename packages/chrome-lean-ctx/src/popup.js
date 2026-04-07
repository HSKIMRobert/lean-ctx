function formatTokens(n) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return `${n}`;
}

async function loadStats() {
  const result = await chrome.storage.local.get(["stats"]);
  const stats = result.stats || { totalSaved: 0, totalCommands: 0 };
  document.getElementById("tokens-saved").textContent = formatTokens(stats.totalSaved);
  document.getElementById("commands").textContent = formatTokens(stats.totalCommands);
}

async function loadSettings() {
  const result = await chrome.storage.local.get(["settings"]);
  const settings = result.settings || { enabled: true, autoCompress: true };
  document.getElementById("toggle-enabled").checked = settings.enabled !== false;
  document.getElementById("toggle-native").checked = settings.autoCompress !== false;
}

function saveSettings() {
  const settings = {
    enabled: document.getElementById("toggle-enabled").checked,
    autoCompress: document.getElementById("toggle-native").checked,
  };
  chrome.storage.local.set({ settings });
}

document.getElementById("toggle-enabled").addEventListener("change", saveSettings);
document.getElementById("toggle-native").addEventListener("change", saveSettings);

loadStats();
loadSettings();
