/**
 * Context Cockpit — full context ledger, field, overlays, and plan.
 */
const VIEW_MODES = [
  'full',
  'map',
  'signatures',
  'diff',
  'aggressive',
  'entropy',
  'lines',
  'reference',
  'handle',
];

function api() {
  return window.LctxApi && window.LctxApi.apiFetch ? window.LctxApi.apiFetch : null;
}

function tip(k) {
  return window.LctxShared && window.LctxShared.tip ? window.LctxShared.tip(k) : '';
}

function fmtLib() {
  return window.LctxFmt || {};
}

function charts() {
  return window.LctxCharts || {};
}

function toast(msg, kind) {
  if (typeof window.showToast === 'function') {
    window.showToast(msg, kind);
    return;
  }
  const t = document.createElement('div');
  t.className = 'toast';
  t.textContent = msg;
  document.body.appendChild(t);
  setTimeout(function () {
    t.remove();
  }, 3000);
}

function targetPath(raw) {
  if (raw == null) return '';
  const s = typeof raw === 'string' ? raw : String(raw);
  return s.startsWith('file:') ? s.slice(5) : s;
}

function formatAuthor(author) {
  if (author == null) return '—';
  if (typeof author === 'string') return author;
  if (author === 'user' || author.user === null) return 'User';
  if (typeof author.user === 'string') return author.user;
  const k = Object.keys(author)[0];
  if (!k) return '—';
  const v = author[k];
  if (k === 'policy') return 'Policy' + (v ? ': ' + v : '');
  if (k === 'agent') return 'Agent' + (v ? ': ' + v : '');
  return k;
}

function formatOperation(op) {
  if (!op || typeof op !== 'object') return String(op);
  const t = op.type;
  switch (t) {
    case 'exclude':
      return 'exclude' + (op.reason ? ' · ' + op.reason : '');
    case 'pin':
      return 'pin' + (op.verbatim === false ? ' (summary)' : '');
    case 'set_view':
      return 'set_view';
    case 'set_priority':
      return (
        'priority ' +
        (op.set_priority != null ? op.set_priority : op['SetPriority'] != null ? op['SetPriority'] : '')
      );
    case 'expire':
      return 'expire (' + (op.after_secs != null ? op.after_secs + 's' : '') + ')';
    case 'rewrite':
      return 'rewrite';
    default:
      return t || JSON.stringify(op);
  }
}

/** Serde may nest SetView as { type, SetView } or flatten — normalize label */
function operationSummary(op) {
  if (!op || typeof op !== 'object') return '';
  if (op.type === 'set_view' && op.set_view != null) return 'set_view → ' + op.set_view;
  if (op.type === 'set_priority' && op.set_priority != null) return 'priority ' + op.set_priority;
  return formatOperation(op);
}

function recommendationCopy(rec) {
  const r = String(rec || '');
  if (r.includes('NoAction')) return 'No action needed — headroom looks OK.';
  if (r.includes('SuggestCompression'))
    return 'Consider switching heavy files to map/signatures or excluding low-value paths.';
  if (r.includes('ForceCompression'))
    return 'Budget is tight: aggressively compress views or remove stale items.';
  if (r.includes('Evict')) return 'Evict stale or low-relevance items to reclaim window space.';
  return r;
}

function gaugeColor(util) {
  const p = util * 100;
  if (p < 60) return 'var(--green)';
  if (p < 80) return 'var(--yellow)';
  return 'var(--red)';
}

function shortenPath(p) {
  if (!p || typeof p !== 'string') return String(p || '');
  const parts = p.split('/');
  if (parts.length <= 3) return p;
  var fnIdx = parts.length - 1;
  var projIdx = -1;
  for (var i = 0; i < parts.length; i++) {
    if (parts[i] === 'src' || parts[i] === 'lib' || parts[i] === 'app' || parts[i] === 'pkg' || parts[i] === 'rust') {
      projIdx = Math.max(0, i - 1);
      break;
    }
  }
  if (projIdx < 0) projIdx = Math.max(0, parts.length - 4);
  return parts.slice(projIdx).join('/');
}

class CockpitContext extends HTMLElement {
  constructor() {
    super();
    this._sortKey = 'path';
    this._sortDir = 'asc';
    this._modeFilter = 'all';
    this._modeMenuOpen = null;
    this._onDocClick = this._onDocClick.bind(this);
    this._onRefresh = this._onRefresh.bind(this);
    this._data = null;
    this._error = null;
    this._loading = true;
  }

  connectedCallback() {
    if (this._ready) return;
    this._ready = true;
    this.style.display = 'block';
    document.addEventListener('click', this._onDocClick);
    document.addEventListener('lctx:refresh', this._onRefresh);
    this.render();
    this.loadData();
  }

  disconnectedCallback() {
    document.removeEventListener('click', this._onDocClick);
    document.removeEventListener('lctx:refresh', this._onRefresh);
    const Ch = charts();
    if (Ch.destroyIfNeeded) Ch.destroyIfNeeded('cockpitCtxModeDist');
  }

  _onRefresh() {
    const v = document.getElementById('view-context');
    if (v && v.classList.contains('active')) this.loadData();
  }

  _onDocClick() {
    if (this._modeMenuOpen) {
      this._modeMenuOpen.classList.remove('open');
      this._modeMenuOpen = null;
    }
  }

  async loadData() {
    const fetchJson = api();
    if (!fetchJson) {
      this._error = 'API client not loaded';
      this._loading = false;
      this.render();
      return;
    }
    this._loading = true;
    this._error = null;
    this.render();

    const paths = [
      '/api/context-ledger',
      '/api/context-field',
      '/api/context-control',
      '/api/context-overlay-history',
      '/api/context-plan',
      '/api/pipeline-stats',
      '/api/intent',
      '/api/session',
      '/api/context-bounce',
      '/api/context-client',
      '/api/context-pressure',
      '/api/context-dynamic-tools',
      '/api/context-radar',
    ];

    const results = await Promise.all(
      paths.map(function (p) {
        return fetchJson(p, { timeoutMs: 12000 }).catch(function (e) {
          return { __error: e && e.error ? e.error : String(e || 'error'), __path: p };
        });
      })
    );

    const [
      ledger,
      field,
      control,
      history,
      plan,
      pipeline,
      intent,
      session,
      bounce,
      clientCaps,
      pressure,
      dynTools,
      radar,
    ] = results;

    const err = [ledger, field, control, plan].find(function (x) {
      return x && x.__error;
    });
    if (err) {
      this._error = String(err.__path) + ': ' + String(err.__error);
    }

    this._data = {
      ledger: ledger && !ledger.__error ? ledger : null,
      field: field && !field.__error ? field : null,
      control: control && !control.__error ? control : null,
      history: Array.isArray(history) ? history : history && history.__error ? [] : history || [],
      plan: plan && !plan.__error ? plan : null,
      pipeline: pipeline && !pipeline.__error ? pipeline : null,
      intent: intent && !intent.__error ? intent : null,
      session: session && !session.__error ? session : null,
      bounce: bounce && !bounce.__error ? bounce : null,
      clientCaps: clientCaps && !clientCaps.__error ? clientCaps : null,
      pressure: pressure && !pressure.__error ? pressure : null,
      dynTools: dynTools && !dynTools.__error ? dynTools : null,
      radar: radar && !radar.__error ? radar : null,
    };

    if (this._data.history && !Array.isArray(this._data.history)) {
      this._data.history = [];
    }

    this._loading = false;
    this.render();
    this._renderModeChart();
  }

  _renderModeChart() {
    const ledger = this._data && this._data.ledger;
    const dist = ledger && ledger.mode_distribution;
    const Ch = charts();
    if (!Ch.doughnutChart || typeof Chart === 'undefined') return;

    const labels = [];
    const values = [];
    if (dist && typeof dist === 'object') {
      for (const k of Object.keys(dist).sort()) {
        labels.push(k);
        values.push(dist[k]);
      }
    }
    if (!labels.length) {
      if (Ch.destroyIfNeeded) Ch.destroyIfNeeded('cockpitCtxModeDist');
      return;
    }
    requestAnimationFrame(function () {
      try {
        Ch.doughnutChart('cockpitCtxModeDist', labels, values);
      } catch (_) {}
    });
  }

  render() {
    const F = fmtLib();
    const esc = F.esc || function (s) { return String(s); };
    const ff = F.ff || function (n) { return String(n); };
    const pc = F.pc || function (a, b) {
      return b > 0 ? Math.round((a / b) * 100) : 0;
    };

    if (this._loading) {
      this.innerHTML =
        '<div class="card"><div class="loading-state">Loading context…</div></div>';
      return;
    }

    if (this._error && !this._data.ledger) {
      this.innerHTML =
        '<div class="card">' +
        '<h3>Error</h3>' +
        '<p class="hs" style="color:var(--red)">' +
        esc(String(this._error)) +
        '</p></div>';
      return;
    }

    const ledger = this._data.ledger;
    const field = this._data.field;
    const control = this._data.control;
    const historyRaw = this._data.history || [];

    let body = '';

    body += this._renderMetrics(ledger, field, F, esc, ff, pc, this._data.session);
    body += this._renderPressureRow(ledger, esc, ff);
    body += this._renderRadarPanel(esc, ff);
    body += this._renderRuntimePanel(esc, ff);
    body += this._renderTableShell(ledger, field, esc, ff, pc);
    body += this._renderOverlays(control, esc);
    body += this._renderPlanExtras(esc);
    body += this._renderHistory(historyRaw, esc);

    this.innerHTML = body;
    this._bindTable();
  }

  _renderRadarPanel(esc, ff) {
    const radar = this._data.radar;
    if (!radar) return '';

    const b = radar.breakdown || {};
    const rules = radar.rules || {};
    const events = radar.recent_events || [];
    const evTotal = radar.events_total || 0;
    const win = b.window_size || 200000;

    function pct(tok) { return win > 0 ? (tok / win * 100).toFixed(1) : '0.0'; }
    function barW(tok) { return Math.min(100, tok / win * 100); }

    const categories = [
      { label: 'System Prompt (Rules)', tokens: b.system_prompt_tokens || 0, color: '#8b5cf6' },
      { label: 'User Messages', tokens: b.user_message_tokens || 0, color: '#3b82f6' },
      { label: 'Agent Responses', tokens: b.agent_response_tokens || 0, color: '#06b6d4' },
      { label: 'lean-ctx Tools', tokens: b.lean_ctx_tool_tokens || 0, color: '#10b981' },
      { label: 'Other MCP Tools', tokens: b.other_mcp_tokens || 0, color: '#f59e0b' },
      { label: 'Native Reads', tokens: b.native_read_tokens || 0, color: '#ef4444' },
      { label: 'Shell Output', tokens: b.shell_tokens || 0, color: '#ec4899' },
    ];

    let html = '<div class="card" style="margin-bottom:20px">';
    html += '<div class="card-header"><h3>Context Radar — Full Budget Breakdown' + tip('context_radar') + '</h3>';
    html += '<span class="badge">' + esc(ff(b.tracked_total || 0)) + ' / ' + esc(ff(win)) + ' tok</span></div>';

    html += '<div style="display:grid;grid-template-columns:1fr 1fr;gap:16px;margin-bottom:16px">';

    html += '<div>';
    for (let i = 0; i < categories.length; i++) {
      const c = categories[i];
      if (c.tokens === 0) continue;
      html += '<div style="margin-bottom:8px">';
      html += '<div style="display:flex;justify-content:space-between;font-size:12px;margin-bottom:2px">';
      html += '<span>' + esc(c.label) + '</span>';
      html += '<span>' + esc(ff(c.tokens)) + ' tok (' + pct(c.tokens) + '%)</span></div>';
      html += '<div style="height:8px;background:var(--bg-3,#1e1e2e);border-radius:4px;overflow:hidden">';
      html += '<div style="width:' + barW(c.tokens) + '%;height:100%;background:' + c.color + ';border-radius:4px"></div>';
      html += '</div></div>';
    }

    const avail = b.available || 0;
    const availPct = pct(avail);
    const availCol = parseFloat(availPct) > 40 ? 'var(--green)' : parseFloat(availPct) > 15 ? 'var(--yellow)' : 'var(--red)';
    html += '<div style="margin-top:12px;padding:10px;background:var(--bg-2,#16161e);border-radius:8px">';
    html += '<div style="display:flex;justify-content:space-between;font-size:13px;font-weight:600">';
    html += '<span>Available</span><span style="color:' + availCol + '">' + esc(ff(avail)) + ' tok (' + availPct + '%)</span>';
    html += '</div></div>';

    if (b.compaction_count > 0) {
      html += '<div class="hs" style="margin-top:8px">Compactions: ' + b.compaction_count + '</div>';
    }
    if (b.thinking_tokens > 0) {
      html += '<div class="hs" style="margin-top:4px">Thinking (not in window): ' + esc(ff(b.thinking_tokens)) + ' tok</div>';
    }
    html += '</div>';

    html += '<div>';
    const ruleFiles = (rules.files || []);
    if (ruleFiles.length > 0) {
      html += '<div style="margin-bottom:16px">';
      html += '<div style="font-size:13px;font-weight:600;margin-bottom:8px">Rules / System Prompt Files</div>';
      html += '<table style="width:100%;font-size:12px"><thead><tr><th style="text-align:left">File</th><th style="text-align:right">Tokens</th></tr></thead><tbody>';
      for (let i = 0; i < ruleFiles.length; i++) {
        const rf = ruleFiles[i];
        html += '<tr><td class="ctx-path-cell" title="' + esc(rf.path) + '">' + esc(shortenPath(rf.path)) + '</td>';
        html += '<td style="text-align:right">' + esc(ff(rf.tokens)) + '</td></tr>';
      }
      html += '</tbody></table>';
      html += '<div class="hs" style="margin-top:4px">Total: ' + esc(ff(rules.total_tokens || 0)) + ' tok</div>';
      html += '</div>';
    }

    if (events.length > 0) {
      html += '<div style="font-size:13px;font-weight:600;margin-bottom:8px">Recent Events (' + evTotal + ' total)</div>';
      html += '<div style="max-height:240px;overflow-y:auto;font-size:11px;font-family:var(--mono,monospace)">';
      for (let i = 0; i < Math.min(events.length, 30); i++) {
        const ev = events[i];
        const typeColors = {
          'user_message': '#3b82f6', 'agent_response': '#06b6d4', 'mcp_call': '#10b981',
          'shell': '#ec4899', 'native_tool': '#ef4444', 'file_read': '#ef4444',
          'thinking': '#a78bfa', 'compaction': '#f59e0b',
        };
        const col = typeColors[ev.event_type] || 'var(--muted)';
        const detail = ev.tool_name || ev.detail || '';
        html += '<div style="display:flex;gap:8px;padding:2px 0;border-bottom:1px solid var(--bg-3,#2a2a3a)">';
        html += '<span style="color:var(--muted);min-width:38px">' + esc(ff(ev.tokens)) + '</span>';
        html += '<span style="color:' + col + ';min-width:80px">' + esc(ev.event_type) + '</span>';
        if (detail) html += '<span style="color:var(--muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + esc(detail) + '</span>';
        html += '</div>';
      }
      html += '</div>';
    } else {
      html += '<p class="hs">No observe events yet. Events are recorded via IDE hooks (lean-ctx hook observe).</p>';
    }
    html += '</div>';

    html += '</div></div>';
    return html;
  }

  _renderRuntimePanel(esc, ff) {
    const bounce = this._data.bounce;
    const caps = this._data.clientCaps;
    const pressure = this._data.pressure;
    const dyn = this._data.dynTools;
    if (!bounce && !caps && !pressure && !dyn) return '';

    let html = '<div class="card"><h3>Runtime Control Plane</h3><div class="grid-3">';

    if (caps) {
      const tier = caps.tier || '?';
      const feats = ['resources', 'prompts', 'elicitation', 'sampling', 'dynamic_tools']
        .filter(function (k) { return caps[k]; });
      html += '<div class="stat-card"><div class="stat-label">IDE</div>' +
        '<div class="stat-value">' + esc(caps.client_id || 'unknown') + '</div>' +
        '<div class="hs">Tier ' + tier + ' · ' + feats.join(', ') +
        (caps.max_tools ? ' · max ' + caps.max_tools + ' tools' : '') + '</div></div>';
    }

    if (pressure) {
      const util = typeof pressure.utilization === 'number' ? pressure.utilization : 0;
      const pct = Math.round(util * 100);
      const color = pct < 60 ? 'var(--green)' : pct < 80 ? 'var(--yellow)' : 'var(--red)';
      const adjSaved = pressure.total_saved_adjusted != null ? pressure.total_saved_adjusted : 0;
      html += '<div class="stat-card"><div class="stat-label">Pressure</div>' +
        '<div class="stat-value" style="color:' + color + '">' + pct + '%</div>' +
        '<div class="hs">' + ff(pressure.remaining_tokens || 0) + ' tok remaining · ' +
        ff(adjSaved) + ' adjusted saved</div></div>';
    }

    if (bounce) {
      html += '<div class="stat-card"><div class="stat-label">Bounce Detection</div>' +
        '<div class="stat-value">' + (bounce.total_bounces || 0) + '</div>' +
        '<div class="hs">' + ff(bounce.total_wasted_tokens || 0) + ' wasted tokens</div></div>';
    }

    if (dyn) {
      const active = dyn.active_categories || [];
      const all = dyn.all_categories || [];
      html += '<div class="stat-card"><div class="stat-label">Dynamic Tools</div>' +
        '<div class="stat-value">' + active.length + '/' + all.length + '</div>' +
        '<div class="hs">' + active.join(', ') +
        (dyn.supports_list_changed ? ' · list_changed' : ' · static') + '</div></div>';
    }

    html += '</div></div>';
    return html;
  }

  _renderMetrics(ledger, field, F, esc, ff, pc, session) {
    const pressure = ledger && ledger.pressure;
    const util = pressure && typeof pressure.utilization === 'number' ? pressure.utilization : 0;
    const rec = pressure && pressure.recommendation != null ? pressure.recommendation : '';
    const win = ledger ? ledger.window_size : 0;
    const temp = field && field.temperature != null ? Number(field.temperature).toFixed(2) : '—';

    const st = session && session.stats ? session.stats : {};
    const tokInput = st.total_tokens_input || 0;
    const tokSaved = st.total_tokens_saved || 0;
    var comprPct = tokInput > 0 ? Math.max(0, Math.min(100, Math.round((tokSaved / tokInput) * 100))) : 0;
    var retainedPct = tokInput > 0 ? Math.round(((tokInput - tokSaved) / tokInput) * 100) : 0;
    var outputTok = tokInput - tokSaved;

    const p100 = util * 100;
    const dash = Math.max(0, Math.min(100, p100));
    const col = gaugeColor(util);
    const circ = 100;
    const off = circ - dash;

    var recLabel = String(rec).replace(/([A-Z])/g, ' $1').trim();
    var recDot = rec === 'NoAction' ? 'var(--green)' : rec === 'SuggestCompression' ? 'var(--yellow)' : 'var(--red)';

    return (
      '<div class="ctx-hero-grid">' +
      '<div class="ctx-gauge-card card">' +
      '<div class="gauge-ring" style="width:120px;height:120px">' +
      '<svg width="120" height="120" viewBox="0 0 36 36" aria-hidden="true">' +
      '<circle class="bg" cx="18" cy="18" r="15.91549430918954" />' +
      '<circle class="fg" cx="18" cy="18" r="15.91549430918954" ' +
      'stroke="' + col + '" ' +
      'stroke-dasharray="' + dash + ' ' + (circ - dash) + '" ' +
      'stroke-dashoffset="' + off + '" />' +
      '</svg>' +
      '<span class="gauge-value">' + Math.round(p100) + '%</span>' +
      '</div>' +
      '<span class="hl" style="margin-top:8px">Token Budget' + tip('token_budget') + '</span>' +
      '<p class="hs">' + esc(ff(win)) + ' window · temp ' + esc(temp) + '</p>' +
      '</div>' +
      '<div class="ctx-metrics-stack">' +
      '<div class="hero r3 stagger">' +
      '<div class="hc">' +
      '<span class="hl">Tokens saved' + tip('tokens_saved') + '</span>' +
      '<div class="hv cockpit-ctx-sparkle" style="color:var(--green)">' + esc(ff(tokSaved)) + '</div>' +
      '<p class="hs">' + esc(ff(tokInput)) + ' input · ' + esc(ff(outputTok)) + ' output</p>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Compression' + tip('compression') + '</span>' +
      '<div class="hv">' + esc(String(comprPct)) + '%</div>' +
      '<p class="hs">' + esc(String(retainedPct)) + '% retained · ' + esc(ff(st.total_tool_calls || 0)) + ' calls</p>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Pressure' + tip('pressure') + '</span>' +
      '<div class="hv" style="font-size:16px"><span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:' + recDot + ';margin-right:6px"></span>' + esc(recLabel) + '</div>' +
      '<p class="hs">' + esc(recommendationCopy(rec)) + '</p>' +
      '</div>' +
      '</div>' +
      '</div>' +
      '</div>'
    );
  }

  _renderPressureRow(ledger, esc, ff) {
    const pressure = ledger && ledger.pressure;
    const util = pressure && typeof pressure.utilization === 'number' ? pressure.utilization : 0;
    const rem = pressure && pressure.remaining_tokens != null ? pressure.remaining_tokens : 0;
    const rec = pressure && pressure.recommendation != null ? pressure.recommendation : '';
    const win = ledger ? ledger.window_size : 0;
    const modeDist = ledger && ledger.mode_distribution;
    const pct = Math.round(util * 100);
    const fillCol =
      pct < 60 ? 'var(--green)' : pct < 80 ? 'var(--yellow)' : 'var(--red)';
    const force = String(rec).includes('ForceCompression');

    let warn = '';
    if (force) {
      warn =
        '<div class="cockpit-ctx-force-warn" role="alert">' +
        '<strong>Budget critical</strong> — force smaller views or exclude low-value files now.' +
        '</div>';
    }

    const hasModes =
      modeDist && typeof modeDist === 'object' && Object.keys(modeDist).length > 0;

    return (
      '<div class="row r12" style="margin-bottom:20px">' +
      '<div class="card">' +
      '<div class="card-header"><h3>Token Pressure' + tip('token_pressure') + '</h3>' +
      '<span class="badge" style="background:' + (pct < 60 ? 'var(--green-dim)' : pct < 80 ? 'var(--yellow-dim)' : 'var(--red-dim)') + ';color:' + (pct < 60 ? 'var(--green)' : pct < 80 ? 'var(--yellow)' : 'var(--red)') + '">' + pct + '%</span></div>' +
      '<div class="pressure-bar" style="height:10px;margin-bottom:12px">' +
      '<div class="pressure-fill" style="width:' + Math.min(100, pct) + '%;background:' + fillCol + '"></div>' +
      '</div>' +
      '<div style="display:grid;grid-template-columns:1fr 1fr;gap:8px">' +
      '<div class="sr"><span class="sl">Remaining</span><span class="sv">' + esc(ff(rem)) + '</span></div>' +
      '<div class="sr"><span class="sl">Budget</span><span class="sv">' + esc(ff(win)) + '</span></div>' +
      '</div>' +
      '<p class="hs" style="margin-top:10px">' + esc(recommendationCopy(rec)) + '</p>' +
      warn +
      '</div>' +
      '<div class="card">' +
      '<div class="card-header"><h3>Mode Distribution' + tip('mode_distribution') + '</h3></div>' +
      (hasModes
        ? '<canvas id="cockpitCtxModeDist" height="180" width="280" aria-label="Mode distribution"></canvas>'
        : '<p class="hs">No ledger entries yet — mode mix appears after reads are recorded.</p>') +
      '</div>' +
      '</div>'
    );
  }

  _renderTableShell(ledger, field, esc, ff, pc) {
    const entries = (ledger && ledger.entries) || [];
    const phiByPath = new Map();
    (field && field.items ? field.items : []).forEach(function (it) {
      if (it && it.path) phiByPath.set(it.path, it.phi);
    });

    const rows = entries.map(function (e) {
      const orig = e.original_tokens != null ? e.original_tokens : 0;
      const sent = e.sent_tokens != null ? e.sent_tokens : 0;
      const savedRow = orig > 0 ? Math.max(0, pc(orig - sent, orig)) : 0;
      const phi =
        e.phi != null
          ? e.phi
          : phiByPath.has(e.path)
            ? phiByPath.get(e.path)
            : null;
      return {
        path: e.path,
        mode:
          e.mode ||
          (typeof e.active_view === 'string' ? e.active_view : '') ||
          'full',
        original_tokens: orig,
        sent_tokens: sent,
        saved_pct: savedRow,
        phi: phi != null ? Number(phi).toFixed(3) : '—',
        raw: e,
      };
    });

    let filtered = rows;
    if (this._modeFilter !== 'all') {
      filtered = rows.filter(function (r) {
        return r.mode === this._modeFilter;
      }, this);
    }

    const sk = this._sortKey;
    const dir = this._sortDir === 'desc' ? -1 : 1;
    const sortDir = this._sortDir;
    filtered.sort(function (a, b) {
      let av = a[sk];
      let bv = b[sk];
      if (sk === 'phi') {
        av = parseFloat(av) || 0;
        bv = parseFloat(bv) || 0;
      }
      if (typeof av === 'string') av = av.toLowerCase();
      if (typeof bv === 'string') bv = bv.toLowerCase();
      if (av < bv) return -1 * dir;
      if (av > bv) return 1 * dir;
      return 0;
    });

    const modes = ['all'];
    rows.forEach(function (r) {
      if (modes.indexOf(r.mode) === -1) modes.push(r.mode);
    });
    modes.sort();

    const th = function (key, label, cls) {
      const active = sk === key;
      const ind = active ? (sortDir === 'asc' ? ' ▲' : ' ▼') : ' ◇';
      return (
        '<th class="' +
        (cls || '') +
        (active ? ' th-sort-active' : '') +
        '" data-sort="' +
        key +
        '" style="cursor:pointer;user-select:none">' +
        label +
        '<span class="sort-ind">' +
        ind +
        '</span></th>'
      );
    };

    const modeOpts = modes
      .map(function (m) {
        return (
          '<option value="' +
          esc(m) +
          '"' +
          (m === this._modeFilter ? ' selected' : '') +
          '>' +
          (m === 'all' ? 'All modes' : esc(m)) +
          '</option>'
        );
      }, this)
      .join('');

    const trs = filtered
      .map(function (r) {
        const pathEsc = esc(r.path);
        const pathData = encodeURIComponent(r.path);
        const selModes = VIEW_MODES.map(function (m) {
          return (
            '<option value="' +
            esc(m) +
            '"' +
            (m === r.mode ? ' selected' : '') +
            '>' +
            esc(m) +
            '</option>'
          );
        }).join('');

        var shortP = shortenPath(r.path);
        var shortEsc = esc(shortP);
        return (
          '<tr>' +
          '<td title="' +
          pathEsc +
          '" class="ctx-path-cell">' +
          shortEsc +
          '</td>' +
          '<td><span class="tag tg">' +
          esc(r.mode) +
          '</span></td>' +
          '<td class="r">' +
          esc(ff(r.original_tokens)) +
          '</td>' +
          '<td class="r">' +
          esc(ff(r.sent_tokens)) +
          '</td>' +
          '<td class="r">' +
          esc(String(r.saved_pct)) +
          '%</td>' +
          '<td class="r">' +
          esc(String(r.phi)) +
          '</td>' +
          '<td style="white-space:nowrap">' +
          '<button type="button" class="action-btn" data-act="pin" data-path="' +
          pathData +
          '">Pin</button> ' +
          '<button type="button" class="action-btn danger" data-act="exclude" data-path="' +
          pathData +
          '">Exclude</button> ' +
          '<button type="button" class="action-btn" data-act="mark_outdated" data-path="' +
          pathData +
          '">Stale</button> ' +
          '<span class="cockpit-ctx-dd" data-path="' +
          pathData +
          '">' +
          '<button type="button" class="action-btn" data-act="mode_toggle">Mode ▾</button>' +
          '<div class="cockpit-ctx-dd-panel">' +
          '<select class="cockpit-ctx-mode-sel" data-path="' +
          pathData +
          '" aria-label="Change view mode">' +
          selModes +
          '</select></div></span>' +
          '</td></tr>'
        );
      })
      .join('');

    return (
      '<div class="card" style="margin-bottom:20px">' +
      '<div class="card-header">' +
      '<h3>Active Context Items' + tip('context_items') + '</h3>' +
      '<div style="display:flex;align-items:center;gap:8px">' +
      '<span class="badge">' + rows.length + '</span>' +
      '<select id="cockpitCtxModeFilter" class="btn" style="padding:4px 8px;font-size:11px">' +
      modeOpts +
      '</select></div></div>' +
      (filtered.length === 0
        ? '<p class="hs" style="padding:12px">No ledger entries for this filter. Context fills as tools record reads.</p>'
        : '<div class="table-scroll"><table><thead><tr>' +
          th('path', 'Path') +
          th('mode', 'Mode') +
          th('original_tokens', 'Original', 'r') +
          th('sent_tokens', 'Sent', 'r') +
          th('saved_pct', 'Saved %', 'r') +
          th('phi', 'Phi', 'r') +
          '<th>Actions</th>' +
          '</tr></thead><tbody>' +
          trs +
          '</tbody></table></div>') +
      '</div>'
    );
  }

  _renderOverlays(control, esc) {
    const list = (control && control.overlays) || [];
    if (!Array.isArray(list)) {
      return (
        '<div class="card" style="margin-bottom:20px">' +
        '<div class="card-header"><h3>Active Overlays' + tip('overlays') + '</h3></div>' +
        '<p class="hs">Could not read overlays.</p></div>'
      );
    }
    if (list.length === 0) {
      return (
        '<div class="card" style="margin-bottom:20px;opacity:.7">' +
        '<div class="card-header"><h3>Active Overlays' + tip('overlays') + '</h3><span class="badge">0</span></div>' +
        '<p class="hs" style="text-align:center;padding:12px 0">No active overlays — use the actions above to pin, exclude, or change file views.</p>' +
        '</div>'
      );
    }

    const cards = list
      .map(function (ov) {
        const path = targetPath(ov.target);
        const pathEsc = esc(path);
        const pathData = encodeURIComponent(path);
        const op = ov.operation;
        const t = op && op.type;
        let undo = '';
        if (t === 'exclude') {
          undo =
            '<button type="button" class="action-btn" data-act="include" data-path="' +
            pathData +
            '">Undo (include)</button>';
        } else if (t === 'pin') {
          undo =
            '<button type="button" class="action-btn" data-act="unpin" data-path="' +
            pathData +
            '">Undo (unpin)</button>';
        }
        const ts =
          ov.created_at != null
            ? esc(String(ov.created_at).replace('T', ' ').slice(0, 19))
            : '—';
        const st = ov.stale ? '<span class="tag td">stale</span> ' : '';
        return (
          '<div class="cockpit-ctx-overlay-card">' +
          st +
          '<div class="cockpit-ctx-oc-path">' +
          pathEsc +
          '</div>' +
          '<div class="cockpit-ctx-oc-meta">' +
          esc(operationSummary(op)) +
          ' · ' +
          esc(formatAuthor(ov.author)) +
          ' · ' +
          ts +
          '</div>' +
          (undo ? '<div style="margin-top:8px">' + undo + '</div>' : '') +
          '</div>'
        );
      })
      .join('');

    return (
      '<div class="card" style="margin-bottom:20px">' +
      '<div class="card-header"><h3>Active Overlays' + tip('overlays') + '</h3></div>' +
      '<div class="cockpit-ctx-overlay-grid">' +
      cards +
      '</div></div>'
    );
  }

  _renderPlanExtras(esc) {
    const plan = this._data.plan;
    const text =
      plan && plan.plan != null && String(plan.plan).trim() !== ''
        ? String(plan.plan)
        : '';

    let planBlock = '';
    if (text) {
      var lines = text.split('\n');
      var header = '';
      var items = [];
      for (var li = 0; li < lines.length; li++) {
        var line = lines[li].trim();
        if (line.startsWith('[ctx_plan]')) {
          header = line.replace('[ctx_plan]', '').trim();
        } else if (line.startsWith('Budget:')) {
          header += (header ? ' · ' : '') + line;
        } else if (line.indexOf('/') > -1 && (line.indexOf(' map ') > -1 || line.indexOf(' full ') > -1 || line.indexOf(' signatures ') > -1 || line.indexOf(' aggressive ') > -1 || line.indexOf(' entropy ') > -1)) {
          items.push(line);
        } else if (line.startsWith('Planned')) {
          // skip heading
        }
      }
      planBlock = '<div class="card" style="margin-bottom:20px">';
      planBlock += '<div class="card-header"><h3>Context Plan' + tip('context_plan') + '</h3></div>';
      if (header) planBlock += '<p class="hs" style="margin-bottom:12px">' + esc(header) + '</p>';
      var warningLine = lines.find(function(l) { return l.trim().startsWith('WARNING:'); });
      if (warningLine) {
        planBlock += '<div style="background:var(--yellow-dim,#fff3cd);color:var(--yellow,#856404);padding:8px 12px;border-radius:6px;margin-bottom:12px;font-size:13px">' + esc(warningLine.trim()) + '</div>';
      }
      var degradedItems = items.filter(function(l) { return l.indexOf('degraded:') > -1; });
      if (degradedItems.length > 0) {
        planBlock += '<div style="background:var(--blue-dim,#cce5ff);color:var(--blue,#004085);padding:8px 12px;border-radius:6px;margin-bottom:12px;font-size:13px">' + degradedItems.length + ' item(s) degraded to fit budget</div>';
      }
      if (items.length > 0) {
        planBlock += '<table><thead><tr><th>Path</th><th>Mode</th><th class="r">Tokens</th><th>Status</th></tr></thead><tbody>';
        for (var pi = 0; pi < items.length; pi++) {
          var m = items[pi].match(/^\s*(\S+)\s+(map|full|signatures|aggressive|entropy|diff|reference|handle|lines:\S+)\s+(\d+)t?\s*(.*)/);
          if (m) {
            var pPath = shortenPath(m[1]);
            var included = m[4] && m[4].indexOf('Included') > -1;
            planBlock += '<tr><td class="ctx-path-cell" title="' + esc(m[1]) + '">' + esc(pPath) + '</td>';
            planBlock += '<td><span class="tag tg">' + esc(m[2]) + '</span></td>';
            planBlock += '<td class="r">' + esc(m[3]) + '</td>';
            planBlock += '<td>' + (included ? '<span class="tag" style="background:var(--green-dim);color:var(--green)">Included</span>' : esc(m[4])) + '</td></tr>';
          }
        }
        planBlock += '</tbody></table>';
      } else {
        planBlock += '<pre class="cockpit-ctx-plan">' + esc(text) + '</pre>';
      }
      planBlock += '</div>';
    } else {
      planBlock =
        '<div class="card" style="margin-bottom:20px">' +
        '<div class="card-header"><h3>Context Plan' + tip('context_plan') + '</h3></div>' +
        '<p class="hs" style="padding:16px">No plan text yet. Run <code>lean-ctx plan</code> to populate the planner.</p>' +
        '</div>';
    }

    const F = fmtLib();
    const ff = F.ff || function (n) { return String(n); };
    const sess = this._data.session;
    const pipe = this._data.pipeline;
    const intent = this._data.intent;

    let sessionBlock = '';
    if (sess) {
      const st = sess.stats || {};
      const toolCalls = st.total_tool_calls || 0;
      const tokSaved = st.total_tokens_saved || 0;
      const tokInput = st.total_tokens_input || 0;
      const filesRead = st.files_read || 0;
      const cmdsRun = st.commands_run || 0;
      const intents = st.intents_inferred || 0;

      sessionBlock += '<div class="card" style="margin-bottom:20px">';
      sessionBlock += '<div class="card-header"><h3>Session' + tip('session') + '</h3>';
      if (sess.id) sessionBlock += '<span class="hs"><code>' + esc(sess.id) + '</code></span>';
      sessionBlock += '</div>';

      sessionBlock += '<div class="hero r4 stagger" style="margin-bottom:16px">';
      sessionBlock += '<div class="hc"><span class="hl">Tool Calls</span><div class="hv">' + esc(ff(toolCalls)) + '</div></div>';
      sessionBlock += '<div class="hc"><span class="hl">Tokens Saved</span><div class="hv" style="color:var(--green)">' + esc(ff(tokSaved)) + '</div></div>';
      sessionBlock += '<div class="hc"><span class="hl">Files Read</span><div class="hv">' + esc(ff(filesRead)) + '</div></div>';
      sessionBlock += '<div class="hc"><span class="hl">Commands</span><div class="hv">' + esc(ff(cmdsRun)) + '</div></div>';
      sessionBlock += '</div>';

      const rows = [];
      if (sess.project_root) rows.push(['Project', shortenPath(sess.project_root)]);
      if (tokInput > 0) rows.push(['Input Tokens', ff(tokInput)]);
      if (intents > 0) rows.push(['Intents Inferred', String(intents)]);
      if (sess.started_at) rows.push(['Started', String(sess.started_at).replace('T', ' ').slice(0, 19)]);
      if (sess.updated_at) rows.push(['Last Active', String(sess.updated_at).replace('T', ' ').slice(0, 19)]);
      if (sess.version) rows.push(['Version', String(sess.version)]);

      if (rows.length > 0) {
        sessionBlock += '<div style="display:grid;grid-template-columns:auto 1fr;gap:8px 20px;font-size:12px;padding:4px 0">';
        for (let i = 0; i < rows.length; i++) {
          sessionBlock += '<span class="sl">' + esc(rows[i][0]) + '</span><span class="sv">' + esc(rows[i][1]) + '</span>';
        }
        sessionBlock += '</div>';
      }
      sessionBlock += '</div>';
    }

    let pipeBlock = '';
    if (pipe && pipe.runs != null) {
      const layers = pipe.per_layer && typeof pipe.per_layer === 'object' ? pipe.per_layer : {};
      const layerKeys = Object.keys(layers);
      pipeBlock += '<div class="card" style="margin-bottom:20px">';
      pipeBlock += '<div class="card-header"><h3>Pipeline' + tip('pipeline') + '</h3><span class="badge">' + pipe.runs + ' run' + (pipe.runs !== 1 ? 's' : '') + '</span></div>';
      if (layerKeys.length > 0) {
        pipeBlock += '<table><thead><tr><th>Layer</th><th class="r">Input Tokens</th><th class="r">Output Tokens</th><th class="r">Duration</th></tr></thead><tbody>';
        for (let i = 0; i < layerKeys.length; i++) {
          const lk = layerKeys[i];
          const lv = layers[lk];
          const dur = lv.total_duration_us ? (lv.total_duration_us / 1000).toFixed(0) + 'ms' : '—';
          pipeBlock += '<tr><td>' + esc(lk) + '</td><td class="r">' + esc(ff(lv.total_input_tokens || 0)) + '</td><td class="r">' + esc(ff(lv.total_output_tokens || 0)) + '</td><td class="r">' + esc(dur) + '</td></tr>';
        }
        pipeBlock += '</tbody></table>';
      }
      pipeBlock += '</div>';
    }

    let intentBlock = '';
    var activeIntent = (sess && sess.active_structured_intent) || (intent && intent.active && intent.intent) || null;
    if (activeIntent && activeIntent.task_type) {
      const it = activeIntent;
      const confPct = it.confidence != null ? Math.round(it.confidence * 100) : null;
      intentBlock += '<div class="card" style="margin-bottom:20px">';
      intentBlock += '<div class="card-header"><h3>Active Intent' + tip('active_intent') + '</h3>';
      intentBlock += '<span class="tag tg">' + esc(it.task_type) + '</span>';
      if (it.scope) intentBlock += '<span class="tag">' + esc(it.scope) + '</span>';
      intentBlock += '</div>';
      if (confPct != null) {
        var confCol = confPct >= 70 ? 'var(--green)' : confPct >= 40 ? 'var(--yellow)' : 'var(--muted)';
        intentBlock += '<div style="display:flex;align-items:center;gap:14px;margin-bottom:12px">';
        intentBlock += '<span class="sl">Confidence</span>';
        intentBlock += '<div class="pressure-bar" style="flex:1;height:8px"><div class="pressure-fill" style="width:' + confPct + '%;background:' + confCol + '"></div></div>';
        intentBlock += '<span class="sv">' + confPct + '%</span></div>';
      }
      if (Array.isArray(it.targets) && it.targets.length > 0) {
        intentBlock += '<p class="sl" style="margin-top:12px;margin-bottom:8px">Targets</p>';
        for (let i = 0; i < Math.min(it.targets.length, 5); i++) {
          intentBlock += '<div class="cockpit-ctx-target-pill">' + esc(shortenPath(it.targets[i])) + '</div>';
        }
        if (it.targets.length > 5) intentBlock += '<span class="hs">+' + (it.targets.length - 5) + ' more</span>';
      }
      intentBlock += '</div>';
    }

    return planBlock + sessionBlock + pipeBlock + intentBlock;
  }

  _renderHistory(historyRaw, esc) {
    let items = Array.isArray(historyRaw) ? historyRaw.slice() : [];
    items.sort(function (a, b) {
      const ta = String(a.created_at || '');
      const tb = String(b.created_at || '');
      return tb.localeCompare(ta);
    });
    items = items.slice(0, 40);

    if (items.length === 0) {
      return (
        '<div class="card">' +
        '<div class="card-header"><h3>Overlay History' + tip('overlay_history') + '</h3></div>' +
        '<p class="hs" style="text-align:center;padding:12px 0;opacity:.6">No overlay operations recorded yet.</p>' +
        '</div>'
      );
    }

    const lines = items
      .map(function (h) {
        const ts =
          h.created_at != null
            ? esc(String(h.created_at).replace('T', ' ').slice(0, 19))
            : '—';
        const path = targetPath(h.target);
        const act = operationSummary(h.operation || {});
        return (
          '<div class="cockpit-ctx-tl-item">' +
          '<div class="cockpit-ctx-tl-dot"></div>' +
          '<div class="cockpit-ctx-tl-body">' +
          '<div class="cockpit-ctx-tl-time">' +
          ts +
          '</div>' +
          '<div class="cockpit-ctx-tl-title">' +
          esc(act) +
          '</div>' +
          '<div class="cockpit-ctx-tl-path">' +
          esc(path) +
          '</div>' +
          '<div class="cockpit-ctx-tl-author">' +
          esc(formatAuthor(h.author)) +
          '</div>' +
          '</div></div>'
        );
      })
      .join('');

    return (
      '<div class="card">' +
      '<div class="card-header"><h3>Overlay History' + tip('overlay_history') + '</h3><span class="badge">' + items.length + '</span></div>' +
      '<div class="cockpit-ctx-timeline">' +
      lines +
      '</div></div>'
    );
  }

  _bindTable() {
    const self = this;
    const ths = this.querySelectorAll('th[data-sort]');
    ths.forEach(function (h) {
      h.addEventListener('click', function () {
        const k = h.getAttribute('data-sort');
        if (self._sortKey === k) {
          self._sortDir = self._sortDir === 'asc' ? 'desc' : 'asc';
        } else {
          self._sortKey = k;
          self._sortDir = 'asc';
        }
        self.render();
        self._renderModeChart();
      });
    });

    const mf = this.querySelector('#cockpitCtxModeFilter');
    if (mf) {
      mf.addEventListener('change', function () {
        self._modeFilter = mf.value || 'all';
        self.render();
        self._renderModeChart();
      });
    }

    this.querySelectorAll('[data-act]').forEach(function (btn) {
      btn.addEventListener('click', async function (e) {
        e.stopPropagation();
        const act = btn.getAttribute('data-act');
        const path = btn.getAttribute('data-path');
        const rawPath = path ? decodeURIComponent(path) : '';
        if (act === 'mode_toggle') {
          const wrap = btn.closest('.cockpit-ctx-dd');
          const panel = wrap && wrap.querySelector('.cockpit-ctx-dd-panel');
          if (panel) {
            const open = panel.classList.toggle('open');
            if (open) self._modeMenuOpen = panel;
            else if (self._modeMenuOpen === panel) self._modeMenuOpen = null;
          }
          return;
        }
        if (rawPath && act) await self._overlayAction(act, rawPath);
      });
    });

    this.querySelectorAll('.cockpit-ctx-mode-sel').forEach(function (sel) {
      sel.addEventListener('change', async function (e) {
        e.stopPropagation();
        const path = sel.getAttribute('data-path');
        const rawPath = path ? decodeURIComponent(path) : '';
        const mode = sel.value;
        if (rawPath && mode) await self.setMode(rawPath, mode);
      });
      sel.addEventListener('click', function (e) {
        e.stopPropagation();
      });
    });
  }

  async _overlayAction(action, path) {
    const fetchJson = api();
    if (!fetchJson) return;
    try {
      await fetchJson('/api/context-overlay', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action, path }),
        timeoutMs: 15000,
      });
      toast(action + ' applied', 'success');
      await this.loadData();
    } catch (err) {
      toast((err && err.error ? err.error : 'Request failed') + '', 'error');
    }
  }

  async pinItem(path) {
    return this._overlayAction('pin', path);
  }

  async excludeItem(path) {
    return this._overlayAction('exclude', path);
  }

  async setMode(path, mode) {
    const fetchJson = api();
    if (!fetchJson) return;
    try {
      await fetchJson('/api/context-overlay', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'set_view', path, value: mode }),
        timeoutMs: 15000,
      });
      toast('View mode updated', 'success');
      await this.loadData();
    } catch (err) {
      toast((err && err.error ? err.error : 'Request failed') + '', 'error');
    }
  }

  async markOutdated(path) {
    return this._overlayAction('mark_outdated', path);
  }
}

customElements.define('cockpit-context', CockpitContext);

export { CockpitContext };
