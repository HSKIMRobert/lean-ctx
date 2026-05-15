/**
 * Sidebar navigation Web Component for Context Cockpit.
 */

const COCKPIT_NAV_SECTIONS = [
  {
    label: null,
    items: [
      { id: 'overview', label: 'Overview', ascii: '[~]' },
    ],
  },
  {
    label: 'Context',
    items: [
      { id: 'context', label: 'Context Manager', ascii: '[##]' },
      { id: 'live', label: 'Live Observatory', ascii: '[**]' },
      { id: 'compression', label: 'Compression Lab', ascii: '[>_]' },
    ],
  },
  {
    label: 'Code Intelligence',
    items: [
      { id: 'deps', label: 'Dependencies', ascii: '[<>]' },
      { id: 'callgraph', label: 'Call Graph', ascii: '[::}' },
      { id: 'symbols', label: 'Symbols', ascii: '[$$]' },
      { id: 'routes', label: 'Routes', ascii: '[/~]' },
      { id: 'search', label: 'Search', ascii: '[??]' },
    ],
  },
  {
    label: 'Knowledge',
    items: [
      { id: 'knowledge', label: 'Knowledge Graph', ascii: '{::}' },
      { id: 'memory', label: 'Memory', ascii: '[db]' },
      { id: 'learning', label: 'Learning', ascii: '[^^]' },
    ],
  },
  {
    label: 'System',
    items: [
      { id: 'agents', label: 'Agents', ascii: '[@_]' },
      { id: 'health', label: 'Health', ascii: '[ok]' },
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
          '<span class="nav-ascii">' + v.ascii + '</span>' +
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
