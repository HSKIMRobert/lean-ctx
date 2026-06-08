/**
 * Architecture report tab — renders the Markdown report from /api/architecture
 * and offers Copy / Download (.md). All data is real (graph signals).
 */

function carchApi() {
  return window.LctxApi && window.LctxApi.apiFetch ? window.LctxApi.apiFetch : null;
}

function carchEsc(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

/** Minimal, safe Markdown → HTML (headings, tables, lists, bold, inline code). */
function carchMarkdown(md) {
  var lines = String(md == null ? '' : md).split('\n');
  var out = [];
  var i = 0;

  function inline(s) {
    var t = carchEsc(s);
    t = t.replace(/`([^`]+)`/g, '<code>$1</code>');
    t = t.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    return t;
  }
  function splitRow(s) {
    return s.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map(function (x) { return x.trim(); });
  }

  while (i < lines.length) {
    var line = lines[i];
    if (/^\s*$/.test(line)) { i++; continue; }

    // Table: header row followed by a |---| separator row.
    if (line.indexOf('|') !== -1 && i + 1 < lines.length &&
      /-/.test(lines[i + 1]) && /^\s*\|?[\s:|-]+\|?\s*$/.test(lines[i + 1])) {
      var header = splitRow(line);
      i += 2;
      var rows = [];
      while (i < lines.length && lines[i].indexOf('|') !== -1 && lines[i].trim() !== '') {
        rows.push(splitRow(lines[i])); i++;
      }
      var t = '<table class="md-table"><thead><tr>';
      header.forEach(function (h) { t += '<th>' + inline(h) + '</th>'; });
      t += '</tr></thead><tbody>';
      rows.forEach(function (r) {
        t += '<tr>';
        r.forEach(function (c) { t += '<td>' + inline(c) + '</td>'; });
        t += '</tr>';
      });
      t += '</tbody></table>';
      out.push(t);
      continue;
    }

    var h = /^(#{1,6})\s+(.*)$/.exec(line);
    if (h) { var lvl = h[1].length; out.push('<h' + lvl + '>' + inline(h[2]) + '</h' + lvl + '>'); i++; continue; }

    if (/^\s*[-*]\s+/.test(line)) {
      var items = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) { items.push(lines[i].replace(/^\s*[-*]\s+/, '')); i++; }
      out.push('<ul>' + items.map(function (it) { return '<li>' + inline(it) + '</li>'; }).join('') + '</ul>');
      continue;
    }
    if (/^\s*\d+\.\s+/.test(line)) {
      var oitems = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) { oitems.push(lines[i].replace(/^\s*\d+\.\s+/, '')); i++; }
      out.push('<ol>' + oitems.map(function (it) { return '<li>' + inline(it) + '</li>'; }).join('') + '</ol>');
      continue;
    }
    if (/^\s*---+\s*$/.test(line)) { out.push('<hr>'); i++; continue; }
    if (/^_.+_$/.test(line.trim())) {
      out.push('<p class="md-meta"><em>' + inline(line.trim().replace(/^_|_$/g, '')) + '</em></p>');
      i++; continue;
    }
    out.push('<p>' + inline(line) + '</p>');
    i++;
  }
  return out.join('\n');
}

class CockpitArchitecture extends HTMLElement {
  connectedCallback() {
    if (this._wired) return;
    this._wired = true;
    this.innerHTML = '<div class="arch-wrap"><div class="arch-loading">Loading architecture report…</div></div>';
  }

  async loadData() {
    var fetchJson = carchApi();
    if (!fetchJson) { this._renderError('API client not loaded'); return; }
    try {
      this._data = await fetchJson('/api/architecture', { timeoutMs: 20000 });
      this._render();
    } catch (e) {
      this._renderError((e && e.error) || 'Failed to load architecture report');
    }
  }

  _renderError(msg) {
    this.innerHTML = '<div class="arch-wrap"><div class="arch-error">' + carchEsc(msg) + '</div></div>';
  }

  _render() {
    var d = this._data || {};
    var md = d.markdown || '';
    var meta = [];
    if (d.file_count != null) meta.push(d.file_count + ' files');
    if (d.edge_count != null) meta.push(d.edge_count + ' edges');
    if (d.community_count != null) meta.push(d.community_count + ' modules');
    if (d.generated_at) meta.push('generated ' + new Date(d.generated_at).toLocaleString());

    this.innerHTML =
      '<div class="arch-wrap">' +
      '<div class="arch-toolbar">' +
      '<button type="button" class="arch-btn" data-arch-copy>Copy Markdown</button>' +
      '<button type="button" class="arch-btn" data-arch-download>Download .md</button>' +
      '<span class="arch-meta">' + carchEsc(meta.join(' \u00b7 ')) + '</span>' +
      '</div>' +
      '<div class="arch-doc markdown-body">' + carchMarkdown(md) + '</div>' +
      '</div>';

    var self = this;
    var copyBtn = this.querySelector('[data-arch-copy]');
    if (copyBtn) copyBtn.addEventListener('click', function () { self._copy(copyBtn); });
    var dlBtn = this.querySelector('[data-arch-download]');
    if (dlBtn) dlBtn.addEventListener('click', function () { self._download(); });
  }

  _copy(btn) {
    var md = (this._data && this._data.markdown) || '';
    var done = function () { btn.textContent = 'Copied \u2713'; setTimeout(function () { btn.textContent = 'Copy Markdown'; }, 1500); };
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(md).then(done, function () {});
    }
  }

  _download() {
    var d = this._data || {};
    var md = d.markdown || '';
    var name = (d.project || 'project') + '-architecture.md';
    var blob = new Blob([md], { type: 'text/markdown' });
    var url = URL.createObjectURL(blob);
    var a = document.createElement('a');
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(function () { URL.revokeObjectURL(url); }, 1000);
  }
}

customElements.define('cockpit-architecture', CockpitArchitecture);
