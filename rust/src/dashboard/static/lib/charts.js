/**
 * Chart.js helpers with LeanCTX dark theme (matches embedded dashboard defaults).
 */

const registry = new Map();

function chartDefaults() {
  const Fmt = window.LctxFmt;
  const fmt = Fmt && Fmt.fmt ? Fmt.fmt : (n) => String(n);
  return {
    responsive: true,
    maintainAspectRatio: true,
    animation: { duration: 500, easing: 'easeOutQuart' },
    plugins: {
      legend: { display: false, labels: { color: '#7a7a9a' } },
    },
    scales: {
      x: {
        ticks: { color: '#7a7a9a', font: { size: 10 } },
        grid: { color: 'rgba(255,255,255,0.03)' },
        border: { display: false },
      },
      y: {
        ticks: {
          color: '#7a7a9a',
          font: { size: 10 },
          callback: function (v) {
            return fmt(v);
          },
        },
        grid: { color: 'rgba(255,255,255,0.03)' },
        border: { display: false },
      },
    },
  };
}

function deepMerge(a, b) {
  if (!b) return a;
  const out = Array.isArray(a) ? a.slice() : Object.assign({}, a);
  for (const k of Object.keys(b)) {
    const bv = b[k],
      av = a[k];
    if (bv && typeof bv === 'object' && !Array.isArray(bv) && av && typeof av === 'object' && !Array.isArray(av)) {
      out[k] = deepMerge(av, bv);
    } else {
      out[k] = bv;
    }
  }
  return out;
}

function destroyIfNeeded(canvasId) {
  const el = typeof canvasId === 'string' ? document.getElementById(canvasId) : canvasId;
  if (!el || el.tagName !== 'CANVAS') return null;
  const existing = registry.get(el.id) || (typeof Chart !== 'undefined' ? Chart.getChart(el) : null);
  if (existing) {
    existing.destroy();
    registry.delete(el.id);
  }
  return el;
}

/**
 * @param {string} canvasId
 * @param {string} type
 * @param {object} data
 * @param {object} [options]
 */
function createChart(canvasId, type, data, options) {
  if (typeof Chart === 'undefined') throw { error: 'Chart.js not loaded' };
  const canvas = destroyIfNeeded(canvasId);
  if (!canvas) throw { error: 'canvas not found: ' + canvasId };
  const defaults = chartDefaults();
  let merged = deepMerge(defaults, options || {});
  if (type === 'doughnut' || type === 'pie') {
    merged = deepMerge(merged, { scales: {} });
    delete merged.scales.x;
    delete merged.scales.y;
  }
  const chart = new Chart(canvas, { type, data, options: merged });
  registry.set(canvas.id, chart);
  return chart;
}

function doughnutChart(canvasId, labels, values, colors) {
  const cols = colors || ['#818cf8', '#38bdf8', '#34d399', '#f472b6', '#fbbf24'];
  return createChart(
    canvasId,
    'doughnut',
    {
      labels,
      datasets: [
        {
          data: values,
          backgroundColor: cols.slice(0, values.length),
          borderWidth: 0,
          hoverOffset: 4,
          borderRadius: 3,
        },
      ],
    },
    {
      cutout: '70%',
      plugins: {
        legend: {
          display: true,
          position: 'bottom',
          labels: { color: '#6b6b88', font: { size: 9 }, padding: 10, usePointStyle: true, pointStyle: 'circle' },
        },
      },
    }
  );
}

function lineChart(canvasId, labels, series, strokeColor, fillRgba) {
  const c = strokeColor || '#34d399';
  const f = fillRgba || 'rgba(52,211,153,.04)';
  return createChart(canvasId, 'line', {
    labels,
    datasets: [
      {
        data: series,
        fill: true,
        borderColor: c,
        backgroundColor: f,
        borderWidth: 2,
        pointRadius: labels.length > 24 ? 0 : 3,
        pointBackgroundColor: c,
        tension: 0.4,
      },
    ],
  });
}

function barChart(canvasId, labels, datasets) {
  return createChart(canvasId, 'bar', { labels, datasets }, { scales: { x: {}, y: {} } });
}

window.LctxCharts = {
  createChart,
  doughnutChart,
  lineChart,
  barChart,
  chartDefaults,
  destroyIfNeeded,
};

export { createChart, doughnutChart, lineChart, barChart, chartDefaults, destroyIfNeeded };
