/**
 * Sidebar navigation Web Component for Context Cockpit.
 */

const NAV_ICONS = {
  overview: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/></svg>',
  context: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>',
  live: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/><line x1="12" y1="2" x2="12" y2="5"/><line x1="12" y1="19" x2="12" y2="22"/><line x1="2" y1="12" x2="5" y2="12"/><line x1="19" y1="12" x2="22" y2="12"/></svg>',
  compression: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>',
  deps: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="5" r="3"/><circle cx="5" cy="19" r="3"/><circle cx="19" cy="19" r="3"/><line x1="12" y1="8" x2="5" y2="16"/><line x1="12" y1="8" x2="19" y2="16"/></svg>',
  callgraph: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 3 21 3 21 8"/><line x1="4" y1="20" x2="21" y2="3"/><polyline points="21 16 21 21 16 21"/><line x1="15" y1="15" x2="21" y2="21"/><line x1="4" y1="4" x2="9" y2="9"/></svg>',
  symbols: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>',
  routes: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M6 9v2a4 4 0 004 4h4a4 4 0 004-4V6"/><line x1="18" y1="3" x2="18" y2="9"/><polyline points="15 6 18 3 21 6"/></svg>',
  search: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>',
  knowledge: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="2"/><circle cx="6" cy="6" r="2"/><circle cx="18" cy="6" r="2"/><circle cx="6" cy="18" r="2"/><circle cx="18" cy="18" r="2"/><line x1="7.8" y1="7.8" x2="10.5" y2="10.5"/><line x1="16.2" y1="7.8" x2="13.5" y2="10.5"/><line x1="7.8" y1="16.2" x2="10.5" y2="13.5"/><line x1="16.2" y1="16.2" x2="13.5" y2="13.5"/></svg>',
  memory: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>',
  learning: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z"/><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z"/></svg>',
  agents: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/></svg>',
  health: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>',
};

const COCKPIT_NAV_SECTIONS = [
  {
    label: null,
    items: [
      { id: 'overview', label: 'Overview' },
    ],
  },
  {
    label: 'Context',
    items: [
      { id: 'context', label: 'Context Manager' },
      { id: 'live', label: 'Live Observatory' },
      { id: 'compression', label: 'Compression Lab' },
    ],
  },
  {
    label: 'Code Intelligence',
    items: [
      { id: 'deps', label: 'Dependencies' },
      { id: 'callgraph', label: 'Call Graph' },
      { id: 'symbols', label: 'Symbols' },
      { id: 'routes', label: 'Routes' },
      { id: 'search', label: 'Search' },
    ],
  },
  {
    label: 'Knowledge',
    items: [
      { id: 'knowledge', label: 'Knowledge Graph' },
      { id: 'memory', label: 'Memory' },
      { id: 'learning', label: 'Learning' },
    ],
  },
  {
    label: 'System',
    items: [
      { id: 'agents', label: 'Agents' },
      { id: 'health', label: 'Health' },
    ],
  },
];

const COCKPIT_VIEWS = COCKPIT_NAV_SECTIONS.reduce(function (acc, section) {
  return acc.concat(section.items);
}, []);

class CockpitNav extends HTMLElement {
  connectedCallback() {
    if (this._ready) return;
    this._ready = true;
    this.style.display = 'contents';
    this._activeId = 'overview';
    this._onViewEvent = this._onViewEvent.bind(this);
    document.addEventListener('lctx:view', this._onViewEvent);
    this.innerHTML =
      '<aside class="sidebar" part="sidebar">' +
      '<div class="sidebar-logo">' +
      '<span style="font-family:var(--mono);font-size:16px;font-weight:700;color:var(--green);flex-shrink:0">&lt;|&gt;</span>' +
      '<span class="sidebar-logo-text">Lean<span>CTX</span></span>' +
      '</div>' +
      '<nav class="sidebar-nav" id="cockpitSidebarNav" role="navigation" aria-label="Cockpit views"></nav>' +
      '<div class="sidebar-footer" id="cockpitSidebarVersion">v---</div>' +
      '</aside>';
    this._nav = this.querySelector('#cockpitSidebarNav');
    this._footer = this.querySelector('#cockpitSidebarVersion');
    this._renderNav();
  }

  disconnectedCallback() {
    document.removeEventListener('lctx:view', this._onViewEvent);
  }

  _onViewEvent(e) {
    const vid = e.detail && e.detail.viewId;
    if (vid) this.setActive(vid);
  }

  _renderNav() {
    const active = this._activeId;
    var html = '';
    for (var si = 0; si < COCKPIT_NAV_SECTIONS.length; si++) {
      var section = COCKPIT_NAV_SECTIONS[si];
      if (si > 0) html += '<div class="nav-divider"></div>';
      if (section.label) {
        html += '<div class="nav-section-label">' + section.label + '</div>';
      }
      html += '<div class="nav-section">';
      for (var ii = 0; ii < section.items.length; ii++) {
        var v = section.items[ii];
        var isActive = v.id === active;
        html +=
          '<div class="nav-item' +
          (isActive ? ' active' : '') +
          '" role="menuitem" data-view="' +
          v.id +
          '" tabindex="0">' +
          '<span class="nav-icon">' + (NAV_ICONS[v.id] || '') + '</span>' +
          '<span class="nav-label">' +
          v.label +
          '</span>' +
          '</div>';
      }
      html += '</div>';
    }
    this._nav.innerHTML = html;
    this._bindItems();
  }

  _emitNavigate(viewId) {
    this.dispatchEvent(
      new CustomEvent('navigate', {
        bubbles: true,
        composed: true,
        detail: { viewId },
      })
    );
  }

  _bindItems() {
    const self = this;
    this._nav.querySelectorAll('.nav-item').forEach(function (item) {
      item.addEventListener('click', function () {
        self._emitNavigate(item.getAttribute('data-view'));
      });
      item.addEventListener('keydown', function (e) {
        const items = [...self._nav.querySelectorAll('.nav-item')];
        const idx = items.indexOf(item);
        if (e.key === 'ArrowDown' && idx < items.length - 1) {
          e.preventDefault();
          items[idx + 1].focus();
        } else if (e.key === 'ArrowUp' && idx > 0) {
          e.preventDefault();
          items[idx - 1].focus();
        } else if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          self._emitNavigate(item.getAttribute('data-view'));
        }
      });
    });
  }

  setActive(viewId) {
    const id = viewId || 'overview';
    this._activeId = id;
    if (!this._nav) return;
    this._nav.querySelectorAll('.nav-item').forEach(function (el) {
      const on = el.getAttribute('data-view') === id;
      el.classList.toggle('active', on);
    });
  }

  setVersion(text) {
    if (this._footer) this._footer.textContent = text;
  }
}

customElements.define('cockpit-nav', CockpitNav);

export { COCKPIT_VIEWS, CockpitNav };
