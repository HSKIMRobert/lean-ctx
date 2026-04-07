const NATIVE_HOST = "com.leanctx.bridge";

let nativePort = null;
let settings = {
  enabled: true,
  autoCompress: true,
  threshold: 500,
};

chrome.storage.local.get(["settings"], (result) => {
  if (result.settings) {
    settings = { ...settings, ...result.settings };
  }
});

chrome.storage.onChanged.addListener((changes) => {
  if (changes.settings) {
    settings = { ...settings, ...changes.settings.newValue };
  }
});

function connectNative() {
  if (nativePort) {
    return nativePort;
  }
  try {
    nativePort = chrome.runtime.connectNative(NATIVE_HOST);
    nativePort.onDisconnect.addListener(() => {
      nativePort = null;
    });
    return nativePort;
  } catch {
    return null;
  }
}

async function compressWithNative(text) {
  return new Promise((resolve) => {
    const port = connectNative();
    if (!port) {
      resolve({ compressed: text, savings: 0, error: "native host not available" });
      return;
    }

    const handler = (response) => {
      port.onMessage.removeListener(handler);
      resolve(response);
    };

    port.onMessage.addListener(handler);
    port.postMessage({ action: "compress", text });

    setTimeout(() => {
      port.onMessage.removeListener(handler);
      resolve({ compressed: text, savings: 0, error: "timeout" });
    }, 5000);
  });
}

function compressFallback(text) {
  let result = text;

  result = result.replace(/\r\n/g, "\n");
  result = result.replace(/\n{3,}/g, "\n\n");
  result = result.replace(/[ \t]+$/gm, "");
  result = result.replace(/^\s*\/\/.*$/gm, "");
  result = result.replace(/^\s*#(?!!).*$/gm, "");

  const inputTokens = estimateTokens(text);
  const outputTokens = estimateTokens(result);
  const savings = inputTokens > 0 ? ((inputTokens - outputTokens) / inputTokens) * 100 : 0;

  return { compressed: result, inputTokens, outputTokens, savings };
}

function estimateTokens(text) {
  return Math.ceil(text.length / 4);
}

chrome.runtime.onMessage.addListener((message, _sender, sendResponse) => {
  if (message.action === "compress") {
    const text = message.text;
    if (!settings.enabled || estimateTokens(text) < settings.threshold) {
      sendResponse({ compressed: text, savings: 0, skipped: true });
      return true;
    }

    compressWithNative(text).then((result) => {
      if (result.error) {
        sendResponse(compressFallback(text));
      } else {
        sendResponse(result);
      }
    });
    return true;
  }

  if (message.action === "getSettings") {
    sendResponse(settings);
    return true;
  }

  if (message.action === "getStats") {
    chrome.storage.local.get(["stats"], (result) => {
      sendResponse(result.stats || { totalSaved: 0, totalCommands: 0 });
    });
    return true;
  }

  return false;
});
