/* cockpit-telemetry.js — Anonymous Telemetry Dashboard */
(function () {
  'use strict';

  var Ch = window.LctxCharts;
  var Fmt = window.LctxFmt;
  var Api = window.LctxApi;

  var esc = Fmt && Fmt.esc ? Fmt.esc : function (s) { return String(s || '').replace(/[&<>"']/g, function (c) { return '&#' + c.charCodeAt(0) + ';'; }); };
  var fmt = Fmt && Fmt.fmt ? Fmt.fmt : function (n) { return n == null ? '—' : Number(n).toLocaleString(); };

  class CockpitTelemetry extends HTMLElement {
    connectedCallback() {
      this._data = null;
      this.innerHTML = '<div class="loading-state"><div class="spinner"></div><p>Loading telemetry data…</p></div>';
      this.loadData();
    }

    async loadData() {
      try {
        var f = Api && Api.apiFetch ? Api.apiFetch : fetch;
        var resp = await f('/api/telemetry');
        this._data = typeof resp.json === 'function' ? await resp.json() : resp;
        this.render();
      } catch (e) {
        this.innerHTML = '<div class="empty-state"><h3>Cannot load telemetry data</h3><p>' + esc(e.message || e) + '</p></div>';
      }
    }

    render() {
      var d = this._data;
      if (!d) return;

      var statusTag = d.enabled
        ? '<span class="tag tg">enabled</span>'
        : '<span class="tag td">disabled</span>';

      var body = '';
      body += '<div class="view-hint">Anonymous opt-in telemetry — no code, no filenames, no personal data.</div>';

      // Hero cards
      body += '<div class="row r4">';
      body += this._heroCard('Status', statusTag, 'telemetry-status');
      body += this._heroCard('Install ID', '<code>' + esc(d.installation_id) + '</code>', 'telemetry-id');
      body += this._heroCard('Total Sent', fmt(d.total_sent), 'telemetry-total');
      body += this._heroCard('Last Heartbeat', esc(d.last_heartbeat || 'never'), 'telemetry-last');
      body += '</div>';

      // Current payload card
      body += '<div class="card">';
      body += '<div class="card-header"><h3>Current Payload</h3><span class="badge">what gets sent</span></div>';
      body += '<pre class="payload-preview">' + esc(JSON.stringify(d.current_payload, null, 2)) + '</pre>';
      body += '</div>';

      // Charts row
      body += '<div class="row r2">';

      // Heartbeat timeline
      body += '<div class="card">';
      body += '<div class="card-header"><h3>Heartbeat Timeline</h3></div>';
      if (d.daily_counts && d.daily_counts.length > 0) {
        body += '<div class="chart-wrap"><canvas id="telem-timeline" height="200"></canvas></div>';
      } else {
        body += '<div class="empty-state sm"><p>No heartbeats sent yet</p></div>';
      }
      body += '</div>';

      // Version distribution
      body += '<div class="card">';
      body += '<div class="card-header"><h3>Version History</h3></div>';
      if (d.distributions && d.distributions.version && d.distributions.version.length > 0) {
        body += '<div class="chart-wrap"><canvas id="telem-versions" height="200"></canvas></div>';
      } else {
        body += '<div class="empty-state sm"><p>No data yet</p></div>';
      }
      body += '</div>';

      body += '</div>';

      // History table
      body += '<div class="card">';
      body += '<div class="card-header"><h3>Heartbeat History</h3><span class="badge">' + fmt(d.total_sent) + ' total</span></div>';
      if (d.history && d.history.length > 0) {
        body += '<div class="table-scroll"><table>';
        body += '<thead><tr><th>Timestamp</th><th>Version</th><th>OS</th><th>Arch</th></tr></thead>';
        body += '<tbody>';
        for (var i = 0; i < d.history.length; i++) {
          var r = d.history[i];
          body += '<tr>';
          body += '<td><code>' + esc(r.timestamp) + '</code></td>';
          body += '<td><span class="tag tb">' + esc(r.version) + '</span></td>';
          body += '<td>' + esc(r.os) + '</td>';
          body += '<td>' + esc(r.arch) + '</td>';
          body += '</tr>';
        }
        body += '</tbody></table></div>';
      } else {
        body += '<div class="empty-state sm"><p>No heartbeats recorded yet. Enable telemetry to start collecting data.</p>';
        body += '<p class="hint">Run: <code>lean-ctx telemetry on</code></p></div>';
      }
      body += '</div>';

      // How to manage
      body += '<div class="card">';
      body += '<div class="card-header"><h3>Manage Telemetry</h3></div>';
      body += '<div class="manage-grid">';
      body += this._cmdRow('Enable', 'lean-ctx telemetry on');
      body += this._cmdRow('Disable', 'lean-ctx telemetry off');
      body += this._cmdRow('Show payload', 'lean-ctx telemetry show');
      body += this._cmdRow('View history', 'lean-ctx telemetry history');
      body += this._cmdRow('Reset install ID', 'lean-ctx telemetry reset-id');
      body += '</div></div>';

      this.innerHTML = body;
      this._renderCharts();
    }

    _heroCard(label, value, id) {
      return '<div class="hero" id="' + id + '">'
        + '<div class="hl">' + label + '</div>'
        + '<div class="hv">' + value + '</div>'
        + '</div>';
    }

    _cmdRow(label, cmd) {
      return '<div class="cmd-row">'
        + '<span class="cmd-label">' + esc(label) + '</span>'
        + '<code class="cmd-code">' + esc(cmd) + '</code>'
        + '</div>';
    }

    _renderCharts() {
      var d = this._data;
      if (!d || !Ch) return;

      var self = this;
      requestAnimationFrame(function () {
        // Timeline chart
        if (d.daily_counts && d.daily_counts.length > 0) {
          var labels = d.daily_counts.map(function (e) { return e.date; });
          var values = d.daily_counts.map(function (e) { return e.count; });
          Ch.destroyIfNeeded('telem-timeline');
          Ch.barChart('telem-timeline', labels, [{ label: 'Heartbeats', data: values, backgroundColor: 'rgba(52,211,153,0.6)', borderColor: '#34d399', borderWidth: 1 }]);
        }

        // Version doughnut
        if (d.distributions && d.distributions.version && d.distributions.version.length > 0) {
          var vLabels = d.distributions.version.map(function (e) { return e.label; });
          var vValues = d.distributions.version.map(function (e) { return e.count; });
          var colors = ['#34d399', '#818cf8', '#38bdf8', '#fbbf24', '#f87171', '#a78bfa', '#fb923c'];
          Ch.destroyIfNeeded('telem-versions');
          Ch.doughnutChart('telem-versions', vLabels, vValues, colors.slice(0, vLabels.length));
        }
      });
    }
  }

  customElements.define('cockpit-telemetry', CockpitTelemetry);

  // Lazy loading: register with the SPA router
  function doRegister() {
    var R = window.LctxRouter;
    if (!R || !R.registerLoader) return;
    R.registerLoader('telemetry', function () {
      var el = document.getElementById('telemetryView');
      if (el && el.loadData) el.loadData();
    });
  }
  if (window.LctxRouter && window.LctxRouter.registerLoader) doRegister();
  else window.addEventListener('lctx:router-ready', doRegister);
})();
