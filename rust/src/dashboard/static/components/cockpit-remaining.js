/**
 * Remaining lightweight views: Route Map.
 * (Trend charts moved into Home — see cockpit-overview.js.)
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

/* ===================== CockpitRoutes ===================== */

class CockpitRoutes extends HTMLElement {
  constructor() {
    super();
    this._loading = true;
    this._error = null;
    this._routes = [];
    this._indexedFileCount = null;
    this._candidateCount = null;
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
      this._indexedFileCount = data && typeof data.indexed_file_count === 'number'
        ? data.indexed_file_count
        : null;
      this._candidateCount = data && typeof data.route_candidate_count === 'number'
        ? data.route_candidate_count
        : null;
    } catch (e) {
      this._error = e && e.error ? e.error : String(e || 'load failed');
      this._routes = [];
      this._indexedFileCount = null;
      this._candidateCount = null;
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
      // Routes come from static analysis of the project's own source code.
      // Be honest about what was scanned and why nothing was found.
      var detail;
      if (this._indexedFileCount === 0) {
        detail =
          'No files are graph-indexed in this project. Routes are detected from the ' +
          'code-map, which only supports specific languages \u2014 see ' +
          '<a href="#deps" style="color:var(--accent)">Dependencies</a> for details.';
      } else if (this._indexedFileCount != null) {
        detail =
          'lean-ctx scanned <b>' + esc(ff(this._candidateCount != null ? this._candidateCount : this._indexedFileCount)) +
          ' source files</b> and found no web-framework route definitions ' +
          '(Express, FastAPI, Flask, Axum, Actix, Spring\u2026). ' +
          'That\u2019s expected for projects that aren\u2019t web APIs \u2014 ' +
          'this view fills up automatically when you work on one.';
      } else {
        detail =
          'Routes are detected from your project\u2019s source code. ' +
          'None were found \u2014 this view fills up automatically for web-API projects.';
      }
      this.innerHTML =
        '<div class="card"><div class="empty-state">' +
        '<h2>No API Routes in This Project</h2>' +
        '<p class="hs" style="color:var(--muted);max-width:520px;margin:8px auto 0">' + detail + '</p>' +
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

customElements.define('cockpit-routes', CockpitRoutes);

(function registerRemLoaders() {
  function doRegister() {
    var R = window.LctxRouter;
    if (!R || !R.registerLoader) return;

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

export { CockpitRoutes };
