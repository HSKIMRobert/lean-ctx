/**
 * Health dashboard — SLOs, Anomalies, Verification, Bug Memory.
 */
var CKH_TABS = [
  { id: 'slos', label: 'SLOs' },
  { id: 'anomalies', label: 'Anomalies' },
  { id: 'verification', label: 'Verification' },
  { id: 'bugmemory', label: 'Bug Memory' },
];

function ckhApi() {
  return window.LctxApi && window.LctxApi.apiFetch ? window.LctxApi.apiFetch : null;
}

function ckhFmt() {
  return window.LctxFmt || {};
}

function ckhCharts() {
  return window.LctxCharts || {};
}

/* ========== component ========== */

class CockpitHealth extends HTMLElement {
  constructor() {
    super();
    this._tab = 'slos';
    this._loading = true;
    this._error = null;
    this._sloData = null;
    this._anomalyData = null;
    this._verificationData = null;
    this._gotchaData = null;
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
    var v = document.getElementById('view-health');
    if (v && v.classList.contains('active')) this.loadData();
  }

  _destroyCharts() {
    var Ch = ckhCharts();
    if (!Ch.destroyIfNeeded) return;
    this.querySelectorAll('canvas[id^="ckh-"]').forEach(function (c) {
      Ch.destroyIfNeeded(c.id);
    });
  }

  /* ---- data ---- */

  async loadData() {
    var fetchJson = ckhApi();
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
      fetchJson('/api/slos', { timeoutMs: 10000 }).catch(function (e) {
        return { __error: e && e.error ? e.error : String(e || 'error') };
      }),
      fetchJson('/api/anomaly', { timeoutMs: 10000 }).catch(function (e) {
        return { __error: e && e.error ? e.error : String(e || 'error') };
      }),
      fetchJson('/api/verification', { timeoutMs: 10000 }).catch(function (e) {
        return { __error: e && e.error ? e.error : String(e || 'error') };
      }),
      fetchJson('/api/gotchas', { timeoutMs: 10000 }).catch(function (e) {
        return { __error: e && e.error ? e.error : String(e || 'error') };
      }),
    ]);

    this._sloData = results[0] && !results[0].__error ? results[0] : null;
    this._anomalyData = results[1] && !results[1].__error ? results[1] : null;
    this._verificationData = results[2] && !results[2].__error ? results[2] : null;
    this._gotchaData = results[3] && !results[3].__error ? results[3] : null;

    if (!this._sloData && !this._anomalyData &&
        !this._verificationData && !this._gotchaData) {
      this._error = 'Could not load health data';
    }

    this._loading = false;
    this.render();
    this._renderSloCharts();
  }

  /* ---- chrome ---- */

  render() {
    var F = ckhFmt();
    var esc = F.esc || function (s) { return String(s); };

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading health data\u2026</div></div>';
      return;
    }
    if (this._error && !this._sloData && !this._anomalyData &&
        !this._verificationData && !this._gotchaData) {
      this.innerHTML =
        '<div class="card"><h3>Error</h3>' +
        '<p class="hs" style="color:var(--red)">' + esc(String(this._error)) + '</p></div>';
      return;
    }

    var body = this._renderTabs(esc);
    body += this._renderTabContent(esc);
    this.innerHTML = body;
    this._bindTabs();
  }

  _renderTabs(esc) {
    var html = '<div class="mode-tabs" id="ckh-tabs">';
    for (var i = 0; i < CKH_TABS.length; i++) {
      var t = CKH_TABS[i];
      html +=
        '<div class="mode-tab' + (t.id === this._tab ? ' active' : '') +
        '" data-ckh-tab="' + t.id + '">' + esc(t.label) + '</div>';
    }
    return html + '</div>';
  }

  _renderTabContent(esc) {
    if (this._tab === 'slos') return this._renderSLOs(esc);
    if (this._tab === 'anomalies') return this._renderAnomalies(esc);
    if (this._tab === 'verification') return this._renderVerification(esc);
    if (this._tab === 'bugmemory') return this._renderBugMemory(esc);
    return '';
  }

  _bindTabs() {
    var self = this;
    this.querySelectorAll('[data-ckh-tab]').forEach(function (tab) {
      tab.addEventListener('click', function () {
        self._destroyCharts();
        self._tab = tab.getAttribute('data-ckh-tab');
        self.render();
        self._renderSloCharts();
      });
    });
  }

  /* ============ SLOs ============ */

  _renderSLOs(esc) {
    var F = ckhFmt();
    var ff = F.ff || function (n) { return String(n); };
    var slo = this._sloData;

    if (!slo || !slo.snapshot || !slo.snapshot.results ||
        slo.snapshot.results.length === 0) {
      return (
        '<div class="card"><div class="empty-state">' +
        '<h2>No SLO Data</h2>' +
        '<p>SLO tracking starts when the daemon monitors service metrics.</p>' +
        '</div></div>'
      );
    }

    var results = slo.snapshot.results;
    var passed = slo.snapshot.passed || 0;
    var total = slo.snapshot.total || results.length;
    var failing = total - passed;

    var summary =
      '<div class="hero r3 stagger" style="margin-bottom:16px">' +
      '<div class="hc"><span class="hl">Total SLOs</span>' +
      '<div class="hv">' + esc(ff(total)) + '</div></div>' +
      '<div class="hc"><span class="hl">Passing</span>' +
      '<div class="hv" style="color:var(--green)">' + esc(ff(passed)) + '</div></div>' +
      '<div class="hc"><span class="hl">Failing</span>' +
      '<div class="hv" style="color:' +
      (failing > 0 ? 'var(--red)' : 'var(--muted)') + '">' +
      esc(ff(failing)) + '</div></div></div>';

    var cards = '<div class="row r3">';
    for (var i = 0; i < results.length; i++) {
      var r = results[i];
      var cls = r.passed ? 'tg' : 'td';
      var label = r.passed ? 'PASS' : 'FAIL';
      var val = r.value != null
        ? (typeof r.value === 'number' ? r.value.toFixed(2) : String(r.value))
        : '\u2014';

      cards +=
        '<div class="card">' +
        '<div class="card-header"><h3>' + esc(r.name || 'SLO ' + (i + 1)) + '</h3>' +
        '<span class="tag ' + cls + '">' + label + '</span></div>' +
        '<div class="sr"><span class="sl">Metric</span>' +
        '<span class="sv">' + esc(r.metric || '\u2014') + '</span></div>' +
        '<div class="sr"><span class="sl">Threshold</span>' +
        '<span class="sv">' + esc(r.threshold != null ? String(r.threshold) : '\u2014') + '</span></div>' +
        '<div class="sr"><span class="sl">Current</span>' +
        '<span class="sv">' + esc(val) + '</span></div>' +
        '<canvas id="ckh-slo-' + i + '" height="80" style="margin-top:12px"></canvas>' +
        '</div>';
    }
    cards += '</div>';
    return summary + cards;
  }

  _renderSloCharts() {
    if (this._tab !== 'slos') return;
    var Ch = ckhCharts();
    if (!Ch.lineChart || typeof Chart === 'undefined') return;
    var slo = this._sloData;
    if (!slo || !slo.history || !Array.isArray(slo.history)) return;

    var results = (slo.snapshot && slo.snapshot.results) || [];
    for (var i = 0; i < results.length; i++) {
      var canvasId = 'ckh-slo-' + i;
      if (!document.getElementById(canvasId)) continue;

      var labels = [];
      var values = [];
      for (var j = 0; j < slo.history.length; j++) {
        var h = slo.history[j];
        labels.push(h.timestamp ? String(h.timestamp).slice(5, 10) : String(j));
        values.push(h.violations != null ? h.violations : 0);
      }
      if (labels.length === 0) continue;

      var color = results[i].passed ? '#34d399' : '#f87171';
      var fill = results[i].passed
        ? 'rgba(52,211,153,.06)' : 'rgba(248,113,113,.06)';
      try { Ch.lineChart(canvasId, labels, values, color, fill); } catch (_) {}
    }
  }

  /* ============ Anomalies ============ */

  _renderAnomalies(esc) {
    var anomalies = this._anomalyData && this._anomalyData.anomalies;
    if (!anomalies || anomalies.length === 0) {
      return (
        '<div class="card"><div class="empty-state">' +
        '<h2>No Anomalies</h2>' +
        '<p>No anomalies detected. System is operating normally.</p>' +
        '</div></div>'
      );
    }

    var sevTag = { critical: 'td', high: 'td', warning: 'ty', medium: 'ty', info: 'tb', low: 'tb' };
    var sevBorder = { critical: 'var(--red)', high: 'var(--red)', warning: 'var(--yellow)', medium: 'var(--yellow)' };

    var html = '<div style="display:flex;flex-direction:column;gap:10px">';
    for (var i = 0; i < anomalies.length; i++) {
      var a = anomalies[i];
      var sev = String(a.severity || '').toLowerCase();
      var cls = sevTag[sev] || 'tb';
      var border = sevBorder[sev] || 'var(--blue)';
      var ts = a.timestamp
        ? String(a.timestamp).replace('T', ' ').slice(0, 19)
        : '\u2014';
      var val = a.value != null
        ? (typeof a.value === 'number' ? a.value.toFixed(2) : String(a.value))
        : '\u2014';
      var exp = a.expected != null
        ? (typeof a.expected === 'number' ? a.expected.toFixed(2) : String(a.expected))
        : '\u2014';

      html +=
        '<div class="card" style="border-left:3px solid ' + border + '">' +
        '<div class="card-header"><h3>' + esc(a.metric || 'Anomaly') + '</h3>' +
        '<span class="tag ' + cls + '">' + esc(a.severity || 'unknown') + '</span></div>' +
        '<div class="sr"><span class="sl">Value</span>' +
        '<span class="sv">' + esc(val) + '</span></div>' +
        '<div class="sr"><span class="sl">Expected</span>' +
        '<span class="sv">' + esc(exp) + '</span></div>' +
        '<div class="sr"><span class="sl">Time</span>' +
        '<span class="sv">' + esc(ts) + '</span></div>' +
        (a.message
          ? '<p class="hs" style="margin-top:10px">' + esc(a.message) + '</p>'
          : '') +
        '</div>';
    }
    return html + '</div>';
  }

  /* ============ Verification ============ */

  _renderVerification(esc) {
    var F = ckhFmt();
    var ff = F.ff || function (n) { return String(n); };
    var v = this._verificationData;

    if (!v || !v.checks || v.checks.length === 0) {
      return (
        '<div class="card"><div class="empty-state">' +
        '<h2>No Verification Data</h2>' +
        '<p>Verification checks appear after running lean-ctx verify.</p>' +
        '</div></div>'
      );
    }

    var total = v.total_checks || v.checks.length;
    var passed = v.passed_checks != null
      ? v.passed_checks
      : (v.checks_passed != null ? v.checks_passed : 0);
    var failing = total - passed;

    var summary =
      '<div class="hero r3 stagger" style="margin-bottom:16px">' +
      '<div class="hc"><span class="hl">Total checks</span>' +
      '<div class="hv">' + esc(ff(total)) + '</div></div>' +
      '<div class="hc"><span class="hl">Passed</span>' +
      '<div class="hv" style="color:var(--green)">' + esc(ff(passed)) + '</div></div>' +
      '<div class="hc"><span class="hl">Failed</span>' +
      '<div class="hv" style="color:' +
      (failing > 0 ? 'var(--red)' : 'var(--muted)') + '">' +
      esc(ff(failing)) + '</div></div></div>';

    var rows = '';
    for (var i = 0; i < v.checks.length; i++) {
      var c = v.checks[i];
      var cls = c.passed ? 'tg' : 'td';
      var label = c.passed ? 'PASS' : 'FAIL';
      rows +=
        '<tr><td>' + esc(c.name || '\u2014') + '</td>' +
        '<td><span class="tag ' + cls + '">' + label + '</span></td>' +
        '<td>' + esc(c.message || '\u2014') + '</td></tr>';
    }

    return (
      summary +
      '<div class="card"><div class="table-scroll"><table>' +
      '<thead><tr><th>Check</th><th>Status</th><th>Message</th></tr></thead>' +
      '<tbody>' + rows + '</tbody></table></div></div>'
    );
  }

  /* ============ Bug Memory ============ */

  _renderBugMemory(esc) {
    var F = ckhFmt();
    var ff = F.ff || function (n) { return String(n); };
    var gotchas = this._gotchaData && this._gotchaData.gotchas;

    if (!gotchas || gotchas.length === 0) {
      return (
        '<div class="card"><div class="empty-state">' +
        '<h2>No Bug Memory</h2>' +
        '<p>Gotchas appear when the system learns from past bugs and mistakes.</p>' +
        '</div></div>'
      );
    }

    var sevTag = { critical: 'td', high: 'td', warning: 'ty', medium: 'ty', info: 'tb', low: 'tb' };
    var rows = '';
    for (var i = 0; i < gotchas.length; i++) {
      var g = gotchas[i];
      var cls = sevTag[String(g.severity || '').toLowerCase()] || 'tb';
      var shortPath = String(g.file_path || '\u2014');
      if (shortPath.length > 35) shortPath = '\u2026' + shortPath.slice(-33);
      var learnedAt = g.learned_at
        ? String(g.learned_at).replace('T', ' ').slice(0, 19)
        : '\u2014';

      rows +=
        '<tr>' +
        '<td><span class="tag ' + cls + '">' + esc(g.severity || '\u2014') + '</span></td>' +
        '<td>' + esc(g.summary || '\u2014') + '</td>' +
        '<td>' + esc(g.category || '\u2014') + '</td>' +
        '<td title="' + esc(g.file_path || '') + '">' + esc(shortPath) + '</td>' +
        '<td class="r">' + esc(String(g.triggered_count != null ? g.triggered_count : '\u2014')) + '</td>' +
        '<td>' + esc(learnedAt) + '</td></tr>';
    }

    return (
      '<div class="card">' +
      '<div class="card-header"><h3>Bug Memory / Gotchas</h3>' +
      '<span class="badge">' + esc(ff(gotchas.length)) + ' learned</span></div>' +
      '<div class="table-scroll"><table>' +
      '<thead><tr><th>Severity</th><th>Summary</th><th>Category</th>' +
      '<th>File</th><th class="r">Count</th><th>Learned</th></tr></thead>' +
      '<tbody>' + rows + '</tbody></table></div></div>'
    );
  }
}

customElements.define('cockpit-health', CockpitHealth);

(function registerCkhLoaders() {
  function doRegister() {
    var R = window.LctxRouter;
    if (!R || !R.registerLoader) return;
    R.registerLoader('health', function () {
      var section = document.getElementById('view-health');
      if (!section) return;
      var el = section.querySelector('cockpit-health');
      if (!el) {
        section.innerHTML = '';
        el = document.createElement('cockpit-health');
        el.id = 'ckh-root';
        section.appendChild(el);
      } else if (typeof el.loadData === 'function') {
        el.loadData();
      }
    });
  }
  if (window.LctxRouter && window.LctxRouter.registerLoader) doRegister();
  else document.addEventListener('DOMContentLoaded', doRegister);
})();

export { CockpitHealth };
