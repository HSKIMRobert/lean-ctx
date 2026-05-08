/**
 * Remaining lightweight views: Learning Curves, Route Map, Context Layer.
 */

/* ===================== shared helpers ===================== */

function remApi() {
  return window.LctxApi && window.LctxApi.apiFetch ? window.LctxApi.apiFetch : null;
}

function remFmt() {
  return window.LctxFmt || {};
}

function remCharts() {
  return window.LctxCharts || {};
}

function remShared() {
  return window.LctxShared || {};
}

/* ===================== CockpitLearning ===================== */

class CockpitLearning extends HTMLElement {
  constructor() {
    super();
    this._loading = true;
    this._error = null;
    this._data = null;
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
    this._destroyCharts();
  }

  _onRefresh() {
    var v = document.getElementById('view-learning');
    if (v && v.classList.contains('active')) this.loadData();
  }

  _destroyCharts() {
    var Ch = remCharts();
    if (!Ch.destroyIfNeeded) return;
    Ch.destroyIfNeeded('ckle-savings');
    Ch.destroyIfNeeded('ckle-compression');
    Ch.destroyIfNeeded('ckle-volume');
  }

  async loadData() {
    var fetchJson = remApi();
    if (!fetchJson) {
      this._error = 'API client not loaded';
      this._loading = false;
      this.render();
      return;
    }
    this._loading = true;
    this._error = null;
    this.render();

    try {
      this._data = await fetchJson('/api/stats', { timeoutMs: 10000 });
    } catch (e) {
      this._error = e && e.error ? e.error : String(e || 'load failed');
      this._data = null;
    }

    this._loading = false;
    this.render();
    this._renderCharts();
  }

  render() {
    var F = remFmt();
    var esc = F.esc || function (s) { return String(s); };

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading learning data\u2026</div></div>';
      return;
    }
    if (this._error && !this._data) {
      this.innerHTML =
        '<div class="card"><h3>Error</h3>' +
        '<p class="hs" style="color:var(--red)">' + esc(String(this._error)) + '</p></div>';
      return;
    }

    this.innerHTML =
      '<div class="row r3">' +
      '<div class="card"><div class="card-header"><h3>Savings Growth</h3></div>' +
      '<canvas id="ckle-savings" height="200"></canvas></div>' +
      '<div class="card"><div class="card-header"><h3>Compression Trend</h3></div>' +
      '<canvas id="ckle-compression" height="200"></canvas></div>' +
      '<div class="card"><div class="card-header"><h3>Command Volume</h3></div>' +
      '<canvas id="ckle-volume" height="200"></canvas></div>' +
      '</div>';

    var S = remShared();
    if (S.injectExpandButtons) S.injectExpandButtons(this);
  }

  _renderCharts() {
    var Ch = remCharts();
    if (!Ch.lineChart || typeof Chart === 'undefined') return;
    var data = this._data;
    if (!data) return;

    var daily = data.daily || [];
    var labels = [];
    var savings = [];
    var compression = [];
    var volume = [];

    for (var i = 0; i < daily.length; i++) {
      var d = daily[i];
      var dateLabel = d.date || d.day || String(i);
      if (typeof dateLabel === 'string' && dateLabel.length > 10) {
        dateLabel = dateLabel.slice(5, 10);
      }
      labels.push(dateLabel);

      var inp = Number(d.input_tokens || d.total_input || 0);
      var out = Number(d.output_tokens || d.total_output || 0);
      savings.push(Math.max(0, inp - out));

      var rate = inp > 0 ? Math.round(((inp - out) / inp) * 100) : 0;
      compression.push(rate);

      volume.push(Number(d.count || d.commands || d.calls || 0));
    }

    if (labels.length === 0) {
      this.innerHTML =
        '<div class="card"><div class="empty-state">' +
        '<h2>No Daily Data Yet</h2>' +
        '<p>Learning curves will appear as lean-ctx records daily usage statistics.</p>' +
        '</div></div>';
      return;
    }

    requestAnimationFrame(function () {
      try {
        Ch.lineChart('ckle-savings', labels, savings,
          '#34d399', 'rgba(52,211,153,.06)');
      } catch (_) {}
      try {
        Ch.lineChart('ckle-compression', labels, compression,
          '#818cf8', 'rgba(129,140,248,.06)');
      } catch (_) {}
      try {
        Ch.lineChart('ckle-volume', labels, volume,
          '#38bdf8', 'rgba(56,189,248,.06)');
      } catch (_) {}
    });
  }
}

/* ===================== CockpitRoutes ===================== */

class CockpitRoutes extends HTMLElement {
  constructor() {
    super();
    this._loading = true;
    this._error = null;
    this._routes = [];
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
    var v = document.getElementById('view-routes');
    if (v && v.classList.contains('active')) this.loadData();
  }

  async loadData() {
    var fetchJson = remApi();
    if (!fetchJson) {
      this._error = 'API client not loaded';
      this._loading = false;
      this.render();
      return;
    }
    this._loading = true;
    this._error = null;
    this.render();

    try {
      var data = await fetchJson('/api/routes', { timeoutMs: 8000 });
      this._routes = (data && data.routes) || (Array.isArray(data) ? data : []);
    } catch (e) {
      this._error = e && e.error ? e.error : String(e || 'load failed');
      this._routes = [];
    }

    this._loading = false;
    this.render();
  }

  render() {
    var F = remFmt();
    var esc = F.esc || function (s) { return String(s); };
    var ff = F.ff || function (n) { return String(n); };

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading routes\u2026</div></div>';
      return;
    }
    if (this._error && this._routes.length === 0) {
      this.innerHTML =
        '<div class="card"><h3>Error</h3>' +
        '<p class="hs" style="color:var(--red)">' + esc(String(this._error)) + '</p></div>';
      return;
    }
    if (this._routes.length === 0) {
      this.innerHTML =
        '<div class="card"><div class="empty-state">' +
        '<h2>No Routes</h2>' +
        '<p>API route data appears after the daemon processes requests.</p>' +
        '</div></div>';
      return;
    }

    var methodColors = {
      GET: 'tg', POST: 'tp', PUT: 'ty', PATCH: 'ty',
      DELETE: 'td', HEAD: 'tb', OPTIONS: 'tb',
    };

    var rows = '';
    for (var i = 0; i < this._routes.length; i++) {
      var r = this._routes[i];
      var method = String(r.method || 'GET').toUpperCase();
      var cls = methodColors[method] || 'tb';
      var count = r.count != null ? ff(r.count) : '\u2014';

      rows +=
        '<tr>' +
        '<td><span class="tag ' + cls + '">' + esc(method) + '</span></td>' +
        '<td style="font-family:var(--mono)">' + esc(r.path || r.route || '\u2014') + '</td>' +
        '<td>' + esc(r.handler || '\u2014') + '</td>' +
        '<td class="r">' + esc(count) + '</td></tr>';
    }

    this.innerHTML =
      '<div class="card">' +
      '<div class="card-header"><h3>API Routes</h3>' +
      '<span class="badge">' + esc(ff(this._routes.length)) + ' routes</span></div>' +
      '<div class="table-scroll"><table>' +
      '<thead><tr><th>Method</th><th>Path</th><th>Handler</th>' +
      '<th class="r">Calls</th></tr></thead>' +
      '<tbody>' + rows + '</tbody></table></div></div>';
  }
}

/* ===================== CockpitContextLayer ===================== */

class CockpitContextLayer extends HTMLElement {
  constructor() {
    super();
    this._loading = true;
    this._error = null;
    this._ledger = null;
    this._field = null;
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
    var v = document.getElementById('view-contextlayer');
    if (v && v.classList.contains('active')) this.loadData();
  }

  async loadData() {
    var fetchJson = remApi();
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
      fetchJson('/api/context-ledger', { timeoutMs: 10000 }).catch(function (e) {
        return { __error: e && e.error ? e.error : String(e || 'error') };
      }),
      fetchJson('/api/context-field', { timeoutMs: 10000 }).catch(function (e) {
        return { __error: e && e.error ? e.error : String(e || 'error') };
      }),
    ]);

    this._ledger = results[0] && !results[0].__error ? results[0] : null;
    this._field = results[1] && !results[1].__error ? results[1] : null;

    if (!this._ledger && !this._field) {
      this._error = 'Could not load context layer data';
    }

    this._loading = false;
    this.render();
  }

  render() {
    var F = remFmt();
    var esc = F.esc || function (s) { return String(s); };
    var ff = F.ff || function (n) { return String(n); };

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading context layer\u2026</div></div>';
      return;
    }
    if (this._error && !this._ledger && !this._field) {
      this.innerHTML =
        '<div class="card"><h3>Error</h3>' +
        '<p class="hs" style="color:var(--red)">' + esc(String(this._error)) + '</p></div>';
      return;
    }

    var body = '';
    body += this._renderMetrics(esc, ff);
    body += this._renderModeDistribution(esc, ff);
    body += this._renderFileList(esc, ff);
    body += this._renderFieldInfo(esc, ff);
    this.innerHTML = body;
  }

  _renderMetrics(esc, ff) {
    var ledger = this._ledger || {};
    var field = this._field || {};
    var entries = ledger.entries || [];
    var totalSent = ledger.total_tokens_sent || 0;
    var totalSaved = ledger.total_tokens_saved || 0;
    var cr = typeof ledger.compression_ratio === 'number' ? ledger.compression_ratio : 1;
    var savedPct = Math.max(0, Math.min(100, Math.round((1 - Math.min(1, cr)) * 100)));

    return (
      '<div class="hero r4 stagger" style="margin-bottom:16px">' +
      '<div class="hc"><span class="hl">Active files</span>' +
      '<div class="hv">' + esc(ff(entries.length)) + '</div></div>' +
      '<div class="hc"><span class="hl">Tokens sent</span>' +
      '<div class="hv">' + esc(ff(totalSent)) + '</div></div>' +
      '<div class="hc"><span class="hl">Tokens saved</span>' +
      '<div class="hv" style="color:var(--green)">' + esc(ff(totalSaved)) + '</div></div>' +
      '<div class="hc"><span class="hl">Compression</span>' +
      '<div class="hv">' + esc(String(savedPct)) + '%</div></div></div>'
    );
  }

  _renderModeDistribution(esc, ff) {
    var ledger = this._ledger || {};
    var modeDist = ledger.mode_distribution;
    if (!modeDist || typeof modeDist !== 'object') return '';

    var modes = Object.keys(modeDist).sort();
    if (modes.length === 0) return '';

    var html = '<div class="card" style="margin-bottom:16px"><h3>View modes in use</h3>';
    for (var i = 0; i < modes.length; i++) {
      var m = modes[i];
      html +=
        '<div class="sr" style="padding:6px 0">' +
        '<span class="sl"><span class="tag tg">' + esc(m) + '</span></span>' +
        '<span class="sv">' + esc(ff(modeDist[m])) + ' files</span></div>';
    }
    return html + '</div>';
  }

  _renderFileList(esc, ff) {
    var ledger = this._ledger || {};
    var entries = ledger.entries || [];

    if (entries.length === 0) {
      return (
        '<div class="card" style="margin-bottom:16px"><h3>Active context files</h3>' +
        '<p class="hs">No files in the context ledger yet.</p></div>'
      );
    }

    var limit = Math.min(entries.length, 30);
    var rows = '';
    for (var i = 0; i < limit; i++) {
      var e = entries[i];
      var path = e.path || '\u2014';
      var shortPath = path.length > 50 ? '\u2026' + path.slice(-48) : path;
      var mode = e.mode || e.active_view || 'full';
      var sent = e.sent_tokens != null ? ff(e.sent_tokens) : '\u2014';

      rows +=
        '<tr>' +
        '<td title="' + esc(path) + '">' + esc(shortPath) + '</td>' +
        '<td><span class="tag tg">' + esc(mode) + '</span></td>' +
        '<td class="r">' + esc(sent) + '</td></tr>';
    }

    return (
      '<div class="card" style="margin-bottom:16px">' +
      '<div class="card-header"><h3>Active context files</h3>' +
      '<span class="badge">' + esc(ff(entries.length)) + '</span></div>' +
      '<div class="table-scroll"><table>' +
      '<thead><tr><th>Path</th><th>Mode</th><th class="r">Tokens</th></tr></thead>' +
      '<tbody>' + rows + '</tbody></table></div></div>'
    );
  }

  _renderFieldInfo(esc, ff) {
    var ledger = this._ledger || {};
    var field = this._field || {};
    var items = field.items || [];
    var temp = field.temperature != null
      ? Number(field.temperature).toFixed(2) : '\u2014';

    return (
      '<div class="card">' +
      '<h3>Field info</h3>' +
      '<div class="sr"><span class="sl">Temperature</span>' +
      '<span class="sv">' + esc(temp) + '</span></div>' +
      '<div class="sr"><span class="sl">Field items</span>' +
      '<span class="sv">' + esc(ff(items.length)) + '</span></div>' +
      '<div class="sr"><span class="sl">Window size</span>' +
      '<span class="sv">' + esc(ff(ledger.window_size || 0)) + '</span></div></div>'
    );
  }
}

/* ===================== register ===================== */

customElements.define('cockpit-learning', CockpitLearning);
customElements.define('cockpit-routes', CockpitRoutes);
customElements.define('cockpit-contextlayer', CockpitContextLayer);

(function registerRemLoaders() {
  function doRegister() {
    var R = window.LctxRouter;
    if (!R || !R.registerLoader) return;

    R.registerLoader('learning', function () {
      var section = document.getElementById('view-learning');
      if (!section) return;
      var el = section.querySelector('cockpit-learning');
      if (!el) {
        section.innerHTML = '';
        el = document.createElement('cockpit-learning');
        el.id = 'ckle-root';
        section.appendChild(el);
      } else if (typeof el.loadData === 'function') {
        el.loadData();
      }
    });

    R.registerLoader('routes', function () {
      var section = document.getElementById('view-routes');
      if (!section) return;
      var el = section.querySelector('cockpit-routes');
      if (!el) {
        section.innerHTML = '';
        el = document.createElement('cockpit-routes');
        el.id = 'ckr-root';
        section.appendChild(el);
      } else if (typeof el.loadData === 'function') {
        el.loadData();
      }
    });

    R.registerLoader('contextlayer', function () {
      var section = document.getElementById('view-contextlayer');
      if (!section) return;
      var el = section.querySelector('cockpit-contextlayer');
      if (!el) {
        section.innerHTML = '';
        el = document.createElement('cockpit-contextlayer');
        el.id = 'ckcl-root';
        section.appendChild(el);
      } else if (typeof el.loadData === 'function') {
        el.loadData();
      }
    });
  }

  if (window.LctxRouter && window.LctxRouter.registerLoader) doRegister();
  else document.addEventListener('DOMContentLoaded', doRegister);
})();

export { CockpitLearning, CockpitRoutes, CockpitContextLayer };
