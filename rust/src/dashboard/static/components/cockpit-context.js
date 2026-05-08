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

    body += this._renderMetrics(ledger, field, F, esc, ff, pc);
    body += this._renderPressureRow(ledger, esc, ff);
    body += this._renderTableShell(ledger, field, esc, ff, pc);
    body += this._renderOverlays(control, esc);
    body += this._renderPlanExtras(esc);
    body += this._renderHistory(historyRaw, esc);

    this.innerHTML = body;
    this._bindTable();
  }

  _renderMetrics(ledger, field, F, esc, ff, pc) {
    const pressure = ledger && ledger.pressure;
    const util = pressure && typeof pressure.utilization === 'number' ? pressure.utilization : 0;
    const rec = pressure && pressure.recommendation != null ? pressure.recommendation : '';
    const cr = ledger && typeof ledger.compression_ratio === 'number' ? ledger.compression_ratio : 1;
    const savedPct = Math.max(0, Math.min(100, Math.round((1 - Math.min(1, cr)) * 100)));
    const sent = ledger ? ledger.total_tokens_sent : 0;
    const saved = ledger ? ledger.total_tokens_saved : 0;
    const win = ledger ? ledger.window_size : 0;
    const temp = field && field.temperature != null ? Number(field.temperature).toFixed(2) : '—';

    const p100 = util * 100;
    const dash = Math.max(0, Math.min(100, p100));
    const col = gaugeColor(util);
    const circ = 100;
    const off = circ - dash;

    return (
      '<div class="hero r4 stagger">' +
      '<div class="hc">' +
      '<span class="hl">Token budget</span>' +
      '<div class="gauge-ring" style="width:96px;height:96px">' +
      '<svg width="96" height="96" viewBox="0 0 36 36" aria-hidden="true">' +
      '<circle class="bg" cx="18" cy="18" r="15.91549430918954" />' +
      '<circle class="fg" cx="18" cy="18" r="15.91549430918954" ' +
      'stroke="' +
      col +
      '" ' +
      'stroke-dasharray="' +
      dash +
      ' ' +
      (circ - dash) +
      '" ' +
      'stroke-dashoffset="' +
      off +
      '" />' +
      '</svg>' +
      '<span class="gauge-value" style="font-size:13px">' +
      Math.round(p100) +
      '%</span>' +
      '</div>' +
      '<p class="hs">' +
      esc(ff(win)) +
      ' window · temp ' +
      esc(temp) +
      '</p>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Tokens saved</span>' +
      '<div class="hv cockpit-ctx-sparkle">' +
      esc(ff(saved)) +
      '</div>' +
      '<p class="hs">cumulative from compression vs original reads</p>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Compression</span>' +
      '<div class="hv">' +
      esc(String(savedPct)) +
      '%</div>' +
      '<p class="hs">~' +
      esc(String(Math.round(cr * 100))) +
      '% of original tokens retained · sent ' +
      esc(ff(sent)) +
      '</p>' +
      '</div>' +
      '<div class="hc">' +
      '<span class="hl">Context pressure</span>' +
      '<div class="hv" style="font-size:16px">' +
      esc(String(rec).replace(/([A-Z])/g, ' $1').trim()) +
      '</div>' +
      '<p class="hs">' +
      esc(recommendationCopy(rec)) +
      '</p>' +
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
      '<div class="row r12" style="margin-bottom:16px">' +
      '<div class="card">' +
      '<div class="card-header"><h3>Token pressure</h3></div>' +
      '<div class="pressure-bar" style="height:12px;margin-bottom:8px">' +
      '<div class="pressure-fill" style="width:' +
      Math.min(100, pct) +
      '%;background:' +
      fillCol +
      '"></div>' +
      '</div>' +
      '<div class="sr" style="padding:4px 0">' +
      '<span class="sl">Remaining</span>' +
      '<span class="sv">' +
      esc(ff(rem)) +
      '</span></div>' +
      '<div class="sr" style="padding:4px 0">' +
      '<span class="sl">Budget</span>' +
      '<span class="sv">' +
      esc(ff(win)) +
      '</span></div>' +
      '<p class="hs" style="margin-top:10px">' +
      esc(recommendationCopy(rec)) +
      '</p>' +
      warn +
      '</div>' +
      '<div class="card">' +
      '<div class="card-header"><h3>Mode distribution</h3></div>' +
      (hasModes
        ? '<canvas id="cockpitCtxModeDist" height="200" width="280" aria-label="Mode distribution"></canvas>'
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
      const savedRow = orig > 0 ? pc(orig - sent, orig) : 0;
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

        return (
          '<tr>' +
          '<td title="' +
          pathEsc +
          '">' +
          pathEsc +
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
      '<div class="card" style="margin-bottom:16px">' +
      '<div class="card-header">' +
      '<h3>Active context items</h3>' +
      '<label class="hs" style="display:flex;align-items:center;gap:8px">Mode ' +
      '<select id="cockpitCtxModeFilter" class="btn" style="padding:4px 8px">' +
      modeOpts +
      '</select></label></div>' +
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
        '<div class="card" style="margin-bottom:16px"><h3>Active overlays</h3>' +
        '<p class="hs">Could not read overlays.</p></div>'
      );
    }
    if (list.length === 0) {
      return (
        '<div class="card" style="margin-bottom:16px"><h3>Active overlays</h3>' +
        '<p class="hs">No active project overlays — pin, exclude, or change views from the table to add some.</p>' +
        (control && control.result
          ? '<pre class="hs" style="margin-top:8px;opacity:.7">' +
            esc(String(control.result)) +
            '</pre>'
          : '') +
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
      '<div class="card" style="margin-bottom:16px">' +
      '<div class="card-header"><h3>Active overlays</h3></div>' +
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
      planBlock =
        '<div class="card" style="margin-bottom:16px">' +
        '<h3>Context plan</h3>' +
        '<pre class="cockpit-ctx-plan">' +
        esc(text) +
        '</pre></div>';
    } else {
      planBlock =
        '<div class="card" style="margin-bottom:16px">' +
        '<h3>Context plan</h3>' +
        '<p class="hs">No plan text yet. Run <code>lean-ctx plan</code> in a project with ledger data, or continue working to populate the planner.</p>' +
        '</div>';
    }

    const bits = [];

    const pipe = this._data.pipeline;
    if (pipe && pipe.runs != null) {
      const layers =
        pipe.per_layer && typeof pipe.per_layer === 'object'
          ? Object.keys(pipe.per_layer).length
          : 0;
      bits.push('Pipeline runs: ' + pipe.runs + (layers ? ' · ' + layers + ' layers' : ''));
    }

    const intent = this._data.intent;
    if (intent && intent.active) {
      bits.push('Structured intent active' + (intent.session_file ? ' · ' + intent.session_file : ''));
    }

    const sess = this._data.session;
    if (sess && (sess.project_root || sess.shell_cwd)) {
      bits.push('Session: ' + (sess.project_root || sess.shell_cwd || ''));
    }

    let extra = '';
    if (bits.length) {
      extra =
        '<div class="card" style="margin-bottom:16px">' +
        '<h3>Session & pipeline</h3>' +
        bits
          .map(function (b) {
            return '<p class="ctx-metric"><span class="ctx-label">●</span><span class="ctx-val">' + esc(b) + '</span></p>';
          })
          .join('') +
        '</div>';
    }

    return planBlock + extra;
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
        '<div class="card"><h3>Overlay history</h3>' +
        '<p class="hs">No overlay operations recorded for this project yet.</p></div>'
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
      '<h3>Overlay history</h3>' +
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
