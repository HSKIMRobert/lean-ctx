/**
 * Dashboard formatting helpers (legacy dashboard parity).
 * @global
 */
(function () {
  const fmt = function (n) {
    if (typeof n !== 'number' || isNaN(n)) return String(n);
    var abs = Math.abs(n);
    if (abs >= 1e9) return (n / 1e9).toFixed(1) + 'B';
    if (abs >= 1e6) return (n / 1e6).toFixed(1) + 'M';
    if (abs >= 1e3) return (n / 1e3).toFixed(1) + 'k';
    return String(n);
  };
  const ff = function (n) {
    if (typeof n !== 'number' || isNaN(n)) return String(n);
    var abs = Math.abs(n);
    if (abs >= 1e9) return (n / 1e9).toFixed(1) + 'B';
    if (abs >= 1e6) return (n / 1e6).toFixed(1) + 'M';
    if (abs >= 1e4) return (n / 1e3).toFixed(1) + 'k';
    if (abs >= 1e3) return (n / 1e3).toFixed(1) + 'k';
    return n.toLocaleString('en-US');
  };
  const pc = function (a, b) {
    if (!Number.isFinite(a) || !Number.isFinite(b) || b <= 0) return 0;
    return Math.round((a / b) * 100);
  };
  const fu = function (a) {
    if (typeof a !== 'number' || !Number.isFinite(a)) return '$0.00';
    return '$' + a.toFixed(2);
  };
  const esc = function (s) {
    const d = document.createElement('div');
    d.textContent = s;
    return d.innerHTML;
  };
  const CM = { i: 2.5, o: 10.0, v: 450, c: 120 };
  const isM = function (n) {
    return String(n).startsWith('ctx_');
  };
  const sb = function (n) {
    return isM(n)
      ? '<span class="tag tp">MCP</span>'
      : '<span class="tag tb">Hook</span>';
  };
  function gc(inp, out, n) {
    const iW = (inp / 1e6) * CM.i,
      iC = (out / 1e6) * CM.i;
    const saved = inp - out;
    const rate = inp > 0 ? saved / inp : 0;
    const eW = n * CM.v;
    const eC = rate > 0.01 ? n * CM.c : eW;
    const oW = (eW / 1e6) * CM.o,
      oC = (eC / 1e6) * CM.o;
    return { iW, iC, oW, oC, tW: iW + oW, tC: iC + oC, sv: iW + oW - iC - oC, os: eW - eC };
  }
  function ss(cmds) {
    const m = { c: 0, i: 0, o: 0, s: 0 },
      h = { c: 0, i: 0, o: 0, s: 0 };
    for (const [name, s] of cmds) {
      const t = isM(name) ? m : h;
      t.c += s.count;
      t.i += s.input_tokens;
      t.o += s.output_tokens;
      t.s += s.input_tokens - s.output_tokens;
    }
    return { m, h };
  }
  function fd(d, r) {
    return !r || r === 0 ? d : d.slice(-r);
  }
  function lv(id, val) {
    const el = document.getElementById(id);
    if (!el) return;
    const s = String(val);
    if (el.textContent === s) return;
    el.textContent = s;
    el.classList.add('flash');
    setTimeout(function () {
      el.classList.remove('flash');
    }, 200);
  }
  window.LctxFmt = {
    fmt,
    ff,
    pc,
    fu,
    esc,
    gc,
    ss,
    fd,
    lv,
    isM,
    sb,
    CM,
  };
})();
