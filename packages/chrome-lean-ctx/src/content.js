const SELECTORS = {
  "chatgpt.com": 'textarea[data-id="root"], div[contenteditable="true"]#prompt-textarea',
  "chat.openai.com": 'textarea[data-id="root"], div[contenteditable="true"]#prompt-textarea',
  "claude.ai": 'div[contenteditable="true"].ProseMirror',
  "gemini.google.com": 'div[contenteditable="true"].ql-editor, rich-textarea .ql-editor',
  "github.com": 'textarea[name="message"], textarea.js-copilot-chat-input',
};

let badge = null;

function getInputSelector() {
  const host = window.location.hostname;
  for (const [domain, selector] of Object.entries(SELECTORS)) {
    if (host.includes(domain)) {
      return selector;
    }
  }
  return null;
}

function createBadge() {
  if (badge) return badge;
  badge = document.createElement("div");
  badge.id = "lean-ctx-badge";
  badge.textContent = "lean-ctx";
  document.body.appendChild(badge);
  return badge;
}

function showSavings(inputTokens, outputTokens, savings) {
  const b = createBadge();
  b.textContent = `lean-ctx: ${inputTokens}→${outputTokens} tok (-${savings.toFixed(0)}%)`;
  b.classList.add("visible");
  setTimeout(() => b.classList.remove("visible"), 4000);
}

function observeInputs() {
  const selector = getInputSelector();
  if (!selector) return;

  document.addEventListener("paste", async (event) => {
    const text = event.clipboardData?.getData("text/plain");
    if (!text || text.length < 200) return;

    const response = await chrome.runtime.sendMessage({
      action: "compress",
      text,
    });

    if (response.skipped || !response.compressed || response.compressed === text) {
      return;
    }

    event.preventDefault();

    const target = document.querySelector(selector);
    if (target) {
      if (target.tagName === "TEXTAREA") {
        const start = target.selectionStart;
        const before = target.value.substring(0, start);
        const after = target.value.substring(target.selectionEnd);
        target.value = before + response.compressed + after;
        target.selectionStart = target.selectionEnd = start + response.compressed.length;
        target.dispatchEvent(new Event("input", { bubbles: true }));
      } else {
        document.execCommand("insertText", false, response.compressed);
      }
    }

    showSavings(
      response.inputTokens || 0,
      response.outputTokens || 0,
      response.savings || 0
    );

    chrome.storage.local.get(["stats"], (result) => {
      const stats = result.stats || { totalSaved: 0, totalCommands: 0 };
      stats.totalSaved += (response.inputTokens || 0) - (response.outputTokens || 0);
      stats.totalCommands += 1;
      chrome.storage.local.set({ stats });
    });
  });
}

observeInputs();
