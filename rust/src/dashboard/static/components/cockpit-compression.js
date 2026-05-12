/**
 * Compression Lab — live compression mode comparison with split-pane preview.
 */
var CKC_MODES = ['full', 'map', 'signatures', 'aggressive', 'entropy'];

function ckcApi() {
  return window.LctxApi && window.LctxApi.apiFetch ? window.LctxApi.apiFetch : null;
}

function ckcFmt() {
  return window.LctxFmt || {};
}

function ckcShared() {
  return window.LctxShared || {};
}

function tip(k) {
  return window.LctxShared && window.LctxShared.tip ? window.LctxShared.tip(k) : '';
}

class CockpitCompression extends HTMLElement {
  constructor() {
    super();
    this._mode = 'map';
    this._files = [];
    this._selectedFile = null;
    this._demoData = null;
    this._loading = true;
    this._demoLoading = false;
    this._error = null;
    this._onRefresh = this._onRefresh.bind(this);
  }

  connectedCallback() {
    if (this._ready) return;
    this._ready = true;
    this.style.display = 'block';
    document.addEventListener('lctx:refresh', this._onRefresh);
    this.render();
    this.loadData();
  }

  disconnectedCallback() {
    document.removeEventListener('lctx:refresh', this._onRefresh);
  }

  _onRefresh() {
    var v = document.getElementById('view-compression');
    if (v && v.classList.contains('active')) this.loadData();
  }

  async loadData() {
    var fetchJson = ckcApi();
    if (!fetchJson) {
      this._error = 'API client not loaded';
      this._loading = false;
      this.render();
      return;
    }
    this._loading = true;
    this._error = null;
    this.render();

    var results = await Promise.all([
      fetchJson('/api/context-ledger', { timeoutMs: 8000 }).catch(function () { return null; }),
      fetchJson('/api/events', { timeoutMs: 8000 }).catch(function () { return null; }),
    ]);

    this._files = this._collectFiles(results[0], results[1]);
    this._loading = false;

    if (this._files.length > 0 && !this._selectedFile) {
      this._selectedFile = this._files[0].path;
      await this._loadDemo();
    } else {
      this.render();
    }
  }

  _collectFiles(ledger, events) {
    var seen = Object.create(null);
    var files = [];

    if (ledger && Array.isArray(ledger.entries)) {
      for (var i = 0; i < ledger.entries.length; i++) {
        var e = ledger.entries[i];
        if (e.path && !seen[e.path]) {
          seen[e.path] = true;
          files.push({
            path: e.path,
            mode: e.active_view || e.mode || 'full',
            original: e.original_tokens || 0,
            sent: e.sent_tokens || 0,
          });
        }
      }
    }

    var evtList = Array.isArray(events) ? events : [];
    for (var j = 0; j < evtList.length; j++) {
      var ev = evtList[j];
      var kind = ev.kind || {};
      if (kind.type === 'ToolCall' && kind.path && !seen[kind.path]) {
        seen[kind.path] = true;
        files.push({
          path: kind.path,
          mode: kind.mode || 'full',
          original: kind.tokens_original || 0,
          sent: 0,
        });
      }
    }

    return files.slice(0, 50);
  }

  async _loadDemo() {
    if (!this._selectedFile) return;
    var fetchJson = ckcApi();
    if (!fetchJson) return;
    this._demoLoading = true;
    this.render();
    try {
      var data = await fetchJson(
        '/api/compression-demo?path=' + encodeURIComponent(this._selectedFile),
        { timeoutMs: 15000 }
      );
      this._demoData = data;
      this._error = null;
    } catch (e) {
      this._demoData = null;
      this._error = e && e.error ? e.error : String(e || 'demo load failed');
    }
    this._demoLoading = false;
    this.render();
  }

  render() {
    var F = ckcFmt();
    var esc = F.esc || function (s) { return String(s); };
    var ff = F.ff || function (n) { return String(n); };
    var S = ckcShared();

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading compression lab\u2026</div></div>';
      return;
    }

    var body = '';
    body += this._renderModeTabs(esc);
    body += this._renderMainLayout(esc, ff);
    body += this._renderHowItWorks();

    this.innerHTML = body;
    this._bind();
    if (S.bindHowItWorks) S.bindHowItWorks(this);
    if (S.injectExpandButtons) S.injectExpandButtons(this);
  }

  _renderModeTabs(esc) {
    var html = '<div class="mode-tabs" id="ckc-mode-tabs">';
    for (var i = 0; i < CKC_MODES.length; i++) {
      var m = CKC_MODES[i];
      html +=
        '<div class="mode-tab' + (m === this._mode ? ' active' : '') +
        '" data-ckc-mode="' + esc(m) + '">' + esc(m) + '</div>';
    }
    html += '</div>';
    return html;
  }

  _renderMainLayout(esc, ff) {
    var html = '<div class="row r12">';

    html +=
      '<div class="card" style="padding:0;overflow:hidden">' +
      '<div style="padding:16px 16px 8px"><h3>Recently read files' + tip('recently_read') + '</h3></div>';

    if (this._files.length === 0) {
      html +=
        '<p class="hs" style="padding:0 16px 16px">' +
        'No files in context yet. Use <code>lean-ctx read</code> to populate.</p>';
    } else {
      html += '<div class="file-list" id="ckc-file-list">';
      for (var i = 0; i < this._files.length; i++) {
        var f = this._files[i];
        var short = f.path.length > 40 ? '\u2026' + f.path.slice(-38) : f.path;
        html +=
          '<div class="file-item' + (f.path === this._selectedFile ? ' selected' : '') +
          '" data-ckc-path="' + esc(f.path) + '" title="' + esc(f.path) + '">' +
          '<span>' + esc(short) + '</span>' +
          '<span class="tag ts" style="margin-left:auto;flex-shrink:0">' + esc(f.mode) + '</span>' +
          '</div>';
      }
      html += '</div>';
    }
    html += '</div>';

    html += '<div class="card">';
    if (this._demoLoading) {
      html += '<div class="loading-state">Compressing\u2026</div>';
    } else if (this._error && !this._demoData) {
      html +=
        '<h3>Compression Demo' + tip('compression_demo') + '</h3>' +
        '<p class="hs" style="color:var(--red)">' + esc(String(this._error)) + '</p>';
    } else if (this._demoData) {
      html += this._renderDemoResult(esc, ff);
    } else {
      html +=
        '<h3>Compression Demo' + tip('compression_demo') + '</h3>' +
        '<p class="hs">Select a file from the left to see compression in action.</p>';
    }
    html += '</div></div>';
    return html;
  }

  _renderDemoResult(esc, ff) {
    var d = this._demoData;
    var origTok = d.original_tokens || 0;
    var origText = d.original || '(empty)';

    var modeData = d.modes && d.modes[this._mode];
    var compTok, compText, savedPct;

    if (this._mode === 'full') {
      compTok = origTok;
      compText = origText;
      savedPct = 0;
    } else if (modeData) {
      compTok = modeData.tokens != null ? modeData.tokens : 0;
      compText = modeData.output || '(empty — mode fully compressed)';
      savedPct = modeData.savings_pct != null ? modeData.savings_pct : 0;
    } else {
      compTok = origTok;
      compText = '(mode not available for this file)';
      savedPct = 0;
    }

    var allModes = d.modes || {};
    var modeKeys = Object.keys(allModes).filter(function (k) { return allModes[k] != null; });
    var comparisonRows = '';
    if (modeKeys.length > 0) {
      modeKeys.sort(function (a, b) {
        var sa = allModes[a].savings_pct || 0;
        var sb = allModes[b].savings_pct || 0;
        return sb - sa;
      });
      for (var i = 0; i < modeKeys.length; i++) {
        var mk = modeKeys[i];
        var mv = allModes[mk];
        var isCurrent = mk === this._mode;
        comparisonRows +=
          '<tr' + (isCurrent ? ' style="background:var(--surface-2)"' : '') + '>' +
          '<td><code>' + esc(mk) + '</code>' + (isCurrent ? ' <span class="tag tg">active</span>' : '') + '</td>' +
          '<td class="r">' + esc(ff(mv.tokens || 0)) + '</td>' +
          '<td class="r">' + esc(String(mv.savings_pct || 0)) + '%</td>' +
          '</tr>';
      }
    }

    return (
      '<div class="hero r3 stagger" style="margin-bottom:16px">' +
      '<div class="hc"><span class="hl">Original</span>' +
      '<div class="hv">' + esc(ff(origTok)) + ' <span class="hs">tokens</span></div></div>' +
      '<div class="hc"><span class="hl">' + esc(this._mode) + '</span>' +
      '<div class="hv">' + esc(ff(compTok)) + ' <span class="hs">tokens</span></div></div>' +
      '<div class="hc"><span class="hl">Savings</span>' +
      '<div class="hv" style="color:var(--green)">' + esc(String(savedPct)) + '%</div></div>' +
      '</div>' +
      (comparisonRows ?
        '<div class="card" style="margin-bottom:16px;padding:12px">' +
        '<h4 style="margin-bottom:8px">All modes comparison' + tip('all_modes_comparison') + '</h4>' +
        '<table><thead><tr><th>Mode</th><th class="r">Tokens</th><th class="r">Savings</th></tr></thead>' +
        '<tbody>' + comparisonRows + '</tbody></table></div>'
        : '') +
      '<div class="split-pane">' +
      '<div class="split-side">' +
      '<h4><span class="tag td">Original</span> ' + esc(ff(origTok)) + ' tokens · ' +
      (d.original_lines || '?') + ' lines</h4>' +
      '<pre>' + esc(String(origText).slice(0, 8000)) + '</pre></div>' +
      '<div class="split-side">' +
      '<h4><span class="tag tg">' + esc(this._mode) + '</span> ' +
      esc(ff(compTok)) + ' tokens</h4>' +
      '<pre>' + esc(String(compText).slice(0, 8000)) + '</pre></div>' +
      '</div>'
    );
  }

  _renderHowItWorks() {
    var S = ckcShared();
    if (!S.howItWorks) return '';
    return S.howItWorks(
      'Compression Modes',
      '<p><strong>full</strong> \u2014 cached verbatim read. Best fidelity, no compression.</p>' +
      '<p><strong>map</strong> \u2014 extracts imports, exports, and API signatures. ' +
      'Great for context files you don\'t edit.</p>' +
      '<p><strong>signatures</strong> \u2014 API surface only (function/class signatures). ' +
      'Minimal tokens.</p>' +
      '<p><strong>aggressive</strong> \u2014 strips comments, blank lines, redundant syntax. ' +
      'Retains logic.</p>' +
      '<p><strong>entropy</strong> \u2014 Shannon entropy + Jaccard similarity filtering. ' +
      'Keeps only high-information lines.</p>'
    );
  }

  _bind() {
    var self = this;
    this.querySelectorAll('[data-ckc-mode]').forEach(function (tab) {
      tab.addEventListener('click', function () {
        var newMode = tab.getAttribute('data-ckc-mode');
        if (newMode === self._mode) return;
        self._mode = newMode;
        if (self._demoData) {
          self.render();
        } else {
          self._loadDemo();
        }
      });
    });

    this.querySelectorAll('[data-ckc-path]').forEach(function (item) {
      item.addEventListener('click', function () {
        var newPath = item.getAttribute('data-ckc-path');
        if (newPath === self._selectedFile) return;
        self._selectedFile = newPath;
        self._demoData = null;
        self._loadDemo();
      });
    });
  }
}

customElements.define('cockpit-compression', CockpitCompression);

(function registerCkcLoaders() {
  function doRegister() {
    var R = window.LctxRouter;
    if (!R || !R.registerLoader) return;
    R.registerLoader('compression', function () {
      var section = document.getElementById('view-compression');
      if (!section) return;
      var el = section.querySelector('cockpit-compression');
      if (!el) {
        section.innerHTML = '';
        el = document.createElement('cockpit-compression');
        el.id = 'ckc-root';
        section.appendChild(el);
      } else if (typeof el.loadData === 'function') {
        el.loadData();
      }
    });
  }
  if (window.LctxRouter && window.LctxRouter.registerLoader) doRegister();
  else document.addEventListener('DOMContentLoaded', doRegister);
})();

export { CockpitCompression };
