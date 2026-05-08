/**
 * Context Cockpit — Agent World: swimlanes, MCP tools, events feed.
 */

function api() {
  return window.LctxApi && window.LctxApi.apiFetch ? window.LctxApi.apiFetch : null;
}

function fmtLib() {
  return window.LctxFmt || {};
}

function relativeTime(iso) {
  if (!iso) return '—';
  var diff = Date.now() - new Date(iso).getTime();
  if (diff < 60000) return 'just now';
  if (diff < 3600000) return Math.floor(diff / 60000) + 'm ago';
  if (diff < 86400000) return Math.floor(diff / 3600000) + 'h ago';
  return Math.floor(diff / 86400000) + 'd ago';
}

function statusDotHtml(status) {
  var s = String(status || '').toLowerCase();
  if (s === 'active' || s === 'running') {
    return '<span class="status-dot active"></span>';
  }
  if (s === 'idle') {
    return '<span class="status-dot warning"></span>';
  }
  return '<span class="status-dot offline"></span>';
}

class CockpitAgents extends HTMLElement {
  constructor() {
    super();
    this._onRefresh = this._onRefresh.bind(this);
    this._data = null;
    this._error = null;
    this._loading = true;
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
    var v = document.getElementById('view-agents');
    if (v && v.classList.contains('active')) this.loadData();
  }

  async loadData() {
    var fetchJson = api();
    if (!fetchJson) {
      this._error = 'API client not loaded';
      this._loading = false;
      this.render();
      return;
    }
    this._loading = true;
    this._error = null;
    this.render();

    var paths = ['/api/agents', '/api/events', '/api/mcp'];
    var results = await Promise.all(
      paths.map(function (p) {
        return fetchJson(p, { timeoutMs: 10000 }).catch(function (e) {
          return { __error: e && e.error ? e.error : String(e || 'error'), __path: p };
        });
      })
    );

    var agents = results[0];
    var events = results[1];
    var mcp = results[2];

    var err = [agents, mcp].find(function (x) {
      return x && x.__error;
    });
    if (err) {
      this._error = String(err.__path) + ': ' + String(err.__error);
    }

    this._data = {
      agents: agents && !agents.__error ? agents : null,
      events: events && !events.__error ? events : null,
      mcp: mcp && !mcp.__error ? mcp : null,
    };

    this._loading = false;
    this.render();
    this._bindEvents();
  }

  render() {
    var F = fmtLib();
    var esc = F.esc || function (s) { return String(s); };
    var ff = F.ff || function (n) { return String(n); };
    var fmt = F.fmt || function (n) { return String(n); };

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading agents…</div></div>';
      return;
    }

    if (this._error && !this._data.agents) {
      this.innerHTML =
        '<div class="card">' +
        '<h3>Error</h3>' +
        '<p class="hs" style="color:var(--red)">' +
        esc(String(this._error)) +
        '</p></div>';
      return;
    }

    var body = '';
    body += this._renderMetrics(esc, ff, fmt);
    body += this._renderSwimlanes(esc, ff, fmt);
    body += this._renderMcpTools(esc, ff);
    body += this._renderEventsFeed(esc);
    this.innerHTML = body;
  }

  _renderMetrics(esc, ff, fmt) {
    var ag = this._data.agents;
    var mcp = this._data.mcp;
    var activeCount = ag && ag.total_active != null ? ag.total_active : 0;
    var sharedCtx = ag && ag.shared_contexts != null ? ag.shared_contexts : 0;

    var toolCalls = 0;
    var tokensSaved = 0;
    var tools = mcp && Array.isArray(mcp.tools) ? mcp.tools : [];
    for (var i = 0; i < tools.length; i++) {
      toolCalls += tools[i].call_count || 0;
      tokensSaved += tools[i].tokens_saved || 0;
    }

    return (
      '<div class="hero r4 stagger">' +
      '<div class="hc">' +
      '<span class="hl">Active Agents</span>' +
      '<div class="hv">' + esc(String(activeCount)) + '</div>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Total Tool Calls</span>' +
      '<div class="hv">' + esc(fmt(toolCalls)) + '</div>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Tokens Saved</span>' +
      '<div class="hv">' + esc(fmt(tokensSaved)) + '</div>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Shared Contexts</span>' +
      '<div class="hv">' + esc(fmt(sharedCtx)) + '</div>' +
      '</div>' +
      '</div>'
    );
  }

  _renderSwimlanes(esc, ff, fmt) {
    var ag = this._data.agents;
    var list = ag && Array.isArray(ag.agents) ? ag.agents : [];

    if (list.length === 0) {
      return (
        '<div class="card" style="margin-bottom:16px">' +
        '<h3>Agent Swimlanes</h3>' +
        '<p class="hs">No agents registered yet. Agents appear here once they connect.</p>' +
        '</div>'
      );
    }

    var cards = list.map(function (a) {
      var dot = statusDotHtml(a.status);
      var name = esc(a.name || a.id || 'Unknown');
      var model = esc(a.model || '—');
      var calls = a.tool_calls != null ? fmt(a.tool_calls) : '0';
      var saved = a.tokens_saved != null ? fmt(a.tokens_saved) : '0';
      var lastAct = relativeTime(a.last_activity);

      return (
        '<div class="swimlane" data-agent-id="' + esc(a.id || '') + '">' +
        '<div class="swimlane-header">' +
        dot +
        '<strong>' + name + '</strong>' +
        '<span class="tag tg" style="margin-left:auto">' + model + '</span>' +
        '</div>' +
        '<div class="swimlane-body">' +
        '<div class="sr"><span class="sl">Tool calls</span><span class="sv">' + esc(calls) + '</span></div>' +
        '<div class="sr"><span class="sl">Tokens saved</span><span class="sv">' + esc(saved) + '</span></div>' +
        '<div class="sr"><span class="sl">Last activity</span><span class="sv">' + esc(lastAct) + '</span></div>' +
        '</div>' +
        '</div>'
      );
    }).join('');

    return (
      '<div class="card" style="margin-bottom:16px">' +
      '<div class="card-header"><h3>Agent Swimlanes</h3></div>' +
      '<div class="cka-swimlane-grid">' + cards + '</div>' +
      '</div>'
    );
  }

  _renderMcpTools(esc, ff) {
    var mcp = this._data.mcp;
    var tools = mcp && Array.isArray(mcp.tools) ? mcp.tools : [];

    if (tools.length === 0) {
      return (
        '<div class="card" style="margin-bottom:16px">' +
        '<h3>MCP Tools</h3>' +
        '<p class="hs">No MCP tools registered.</p>' +
        '</div>'
      );
    }

    var activeTools = tools.filter(function (t) { return t.call_count > 0; });
    var inactiveTools = tools.filter(function (t) { return !t.call_count; });

    var rows = activeTools.concat(inactiveTools).map(function (t) {
      var name = typeof t === 'string' ? t : (t.name || t.id || '—');
      var desc = typeof t === 'object' && t.description ? t.description : '';
      var calls = typeof t === 'object' && t.call_count != null ? ff(t.call_count) : '—';
      var saved = typeof t === 'object' && t.tokens_saved ? ff(t.tokens_saved) : '—';
      var active = t.call_count > 0;
      return (
        '<tr' + (active ? '' : ' style="opacity:0.5"') + '>' +
        '<td><code>' + esc(name) + '</code></td>' +
        '<td>' + esc(desc) + '</td>' +
        '<td class="r">' + esc(calls) + '</td>' +
        '<td class="r">' + esc(saved) + '</td>' +
        '</tr>'
      );
    }).join('');

    return (
      '<div class="card" style="margin-bottom:16px">' +
      '<div class="card-header"><h3>MCP Tools</h3></div>' +
      '<div class="table-scroll"><table>' +
      '<thead><tr><th>Tool</th><th>Description</th><th class="r">Calls</th><th class="r">Tokens Saved</th></tr></thead>' +
      '<tbody>' + rows + '</tbody>' +
      '</table></div></div>'
    );
  }

  _renderEventsFeed(esc) {
    var F = fmtLib();
    var ff = F.ff || function (n) { return String(n); };
    var evts = this._data.events;
    var list = [];
    if (Array.isArray(evts)) {
      list = evts;
    } else if (evts && Array.isArray(evts.events)) {
      list = evts.events;
    }

    list = list.slice().sort(function (a, b) {
      var ta = a.timestamp || a.created_at || '';
      var tb = b.timestamp || b.created_at || '';
      return ta > tb ? -1 : ta < tb ? 1 : 0;
    }).slice(0, 30);

    if (list.length === 0) {
      return (
        '<div class="card">' +
        '<h3>Recent Events</h3>' +
        '<p class="hs">No events recorded yet.</p>' +
        '</div>'
      );
    }

    var items = list.map(function (ev) {
      var ts = ev.timestamp || ev.created_at || '';
      var tsDisplay = ts ? esc(String(ts).replace('T', ' ').slice(0, 19)) : '—';

      var kind = ev.kind || {};
      var evType = kind.type || ev.type || 'Unknown';
      var tool = kind.tool || '';
      var path = kind.path || '';
      var mode = kind.mode || '';
      var saved = kind.tokens_saved;
      var original = kind.tokens_original;
      var durationMs = kind.duration_ms;

      var label = esc(evType);
      if (tool) label += ' <code>' + esc(tool) + '</code>';
      if (path) label += ' <span class="hs">' + esc(path) + '</span>';
      if (mode) label += ' <span class="tag ts">' + esc(mode) + '</span>';

      var stats = '';
      if (saved != null && saved > 0) {
        stats += '<span class="tag tg">-' + esc(ff(saved)) + ' tok</span> ';
      }
      if (original != null) {
        stats += '<span class="hs">' + esc(ff(original)) + ' orig</span> ';
      }
      if (durationMs != null && durationMs > 0) {
        stats += '<span class="hs">' + durationMs + 'ms</span>';
      }

      return (
        '<div class="cka-event-item">' +
        '<div class="cka-event-time">' + tsDisplay + '</div>' +
        '<div class="cka-event-body">' + label +
        (stats ? '<div style="margin-top:2px">' + stats + '</div>' : '') +
        '</div>' +
        '</div>'
      );
    }).join('');

    return (
      '<div class="card">' +
      '<div class="card-header"><h3>Recent Events</h3></div>' +
      '<div class="cka-events-feed">' + items + '</div>' +
      '</div>'
    );
  }

  _bindEvents() {
    var self = this;
    this.querySelectorAll('.swimlane[data-agent-id]').forEach(function (el) {
      el.addEventListener('click', function () {
        el.classList.toggle('swimlane--expanded');
      });
    });
  }
}

customElements.define('cockpit-agents', CockpitAgents);

(function () {
  function reg() {
    if (window.LctxRouter && window.LctxRouter.registerLoader) {
      window.LctxRouter.registerLoader('agents', function () {
        var el = document.querySelector('cockpit-agents');
        if (el && typeof el.loadData === 'function') return el.loadData();
      });
    }
  }
  if (window.LctxRouter && window.LctxRouter.registerLoader) reg();
  else document.addEventListener('DOMContentLoaded', reg);
})();

export { CockpitAgents };
