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

function tip(k) {
  return window.LctxShared && window.LctxShared.tip ? window.LctxShared.tip(k) : '';
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
      var cached = window.LctxApi && window.LctxApi.cachedFetch ? window.LctxApi.cachedFetch : fetchJson;
      this._data = await cached('/api/stats', { timeoutMs: 10000 });
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
      '<div class="card"><div class="card-header"><h3>Savings Growth' + tip('savings_growth') + '</h3></div>' +
      '<canvas id="ckle-savings" height="200"></canvas></div>' +
      '<div class="card"><div class="card-header"><h3>Compression Trend' + tip('compression_trend') + '</h3></div>' +
      '<canvas id="ckle-compression" height="200"></canvas></div>' +
      '<div class="card"><div class="card-header"><h3>Command Volume' + tip('command_volume') + '</h3></div>' +
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
      '<div class="card-header"><h3>API Routes' + tip('routes_table') + '</h3>' +
      '<span class="badge">' + esc(ff(this._routes.length)) + ' routes</span></div>' +
      '<div class="table-scroll"><table>' +
      '<thead><tr><th>Method</th><th>Path</th><th>Handler</th>' +
      '<th class="r">Calls</th></tr></thead>' +
      '<tbody>' + rows + '</tbody></table></div></div>';
  }
}

/* ===================== register ===================== */

customElements.define('cockpit-learning', CockpitLearning);
customElements.define('cockpit-routes', CockpitRoutes);

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
  }

  if (window.LctxRouter && window.LctxRouter.registerLoader) doRegister();
  else document.addEventListener('DOMContentLoaded', doRegister);
})();

export { CockpitLearning, CockpitRoutes };
