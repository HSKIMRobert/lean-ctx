/**
 * Dashboard Tour (#295) — a step-by-step intro overlay that highlights
 * key features on first visit. Stores completion in localStorage.
 */
(function () {
  'use strict';

  var STORAGE_KEY = 'leanctx_tour_done';
  var STEPS = [
    {
      target: '.graph-stats',
      title: 'Graph Overview',
      body: 'This bar shows file/edge counts and quick toggles for edge visibility, community hulls, and the meta-graph view.',
      position: 'below'
    },
    {
      target: '#ckg-deps-legend',
      title: 'Interactive Legend',
      body: 'Click a language to filter. Click "all" to reset. The graph instantly reflects your selection.',
      position: 'below'
    },
    {
      target: '#ckg-deps-layers',
      title: 'Layers Panel',
      body: 'Toggle individual edge types on/off: imports, calls, co-access, community links. Combine with "hide weak" for focused views.',
      position: 'below'
    },
    {
      target: '.graph-search',
      title: 'Search & Focus',
      body: 'Type a filename to highlight it. Press Enter or click a result to zoom in. Matching nodes glow.',
      position: 'below'
    },
    {
      target: '#ckg-insights',
      title: 'Insights & Suggested Questions',
      body: 'Automated analysis: god-nodes, cycles, surprising connections, community cohesion. Click a question to explore.',
      position: 'left'
    },
    {
      target: '.graph-inspector',
      title: 'Inspector Panel',
      body: 'Click any node to open the inspector: neighbors, dependency paths, and impact radius at a glance.',
      position: 'left'
    }
  ];

  function createOverlay() {
    var el = document.createElement('div');
    el.className = 'tour-overlay';
    el.id = 'leanctx-tour-overlay';
    el.innerHTML =
      '<div class="tour-backdrop"></div>' +
      '<div class="tour-box">' +
      '<div class="tour-header"><span class="tour-step-num"></span><button class="tour-close" aria-label="Close tour">&times;</button></div>' +
      '<h3 class="tour-title"></h3>' +
      '<p class="tour-body"></p>' +
      '<div class="tour-nav">' +
      '<button class="tour-prev">Back</button>' +
      '<button class="tour-next">Next</button>' +
      '</div></div>';
    document.body.appendChild(el);
    return el;
  }

  function positionBox(box, targetEl, position) {
    if (!targetEl) {
      box.style.top = '50%';
      box.style.left = '50%';
      box.style.transform = 'translate(-50%, -50%)';
      return;
    }
    var rect = targetEl.getBoundingClientRect();
    box.style.transform = '';
    if (position === 'below') {
      box.style.top = (rect.bottom + 12) + 'px';
      box.style.left = Math.max(12, rect.left) + 'px';
    } else if (position === 'left') {
      box.style.top = rect.top + 'px';
      box.style.left = Math.max(12, rect.left - box.offsetWidth - 12) + 'px';
    } else {
      box.style.top = (rect.bottom + 12) + 'px';
      box.style.left = rect.left + 'px';
    }
  }

  function runTour(containerEl) {
    var overlay = createOverlay();
    var box = overlay.querySelector('.tour-box');
    var stepNum = overlay.querySelector('.tour-step-num');
    var title = overlay.querySelector('.tour-title');
    var body = overlay.querySelector('.tour-body');
    var prevBtn = overlay.querySelector('.tour-prev');
    var nextBtn = overlay.querySelector('.tour-next');
    var closeBtn = overlay.querySelector('.tour-close');
    var currentStep = 0;

    function show(i) {
      currentStep = i;
      var step = STEPS[i];
      stepNum.textContent = (i + 1) + ' / ' + STEPS.length;
      title.textContent = step.title;
      body.textContent = step.body;
      prevBtn.disabled = i === 0;
      nextBtn.textContent = i === STEPS.length - 1 ? 'Done' : 'Next';
      var target = containerEl.querySelector(step.target);
      positionBox(box, target, step.position);
      if (target) {
        target.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
        target.classList.add('tour-highlight');
      }
      STEPS.forEach(function (s, j) {
        if (j !== i) {
          var el = containerEl.querySelector(s.target);
          if (el) el.classList.remove('tour-highlight');
        }
      });
    }

    function finish() {
      localStorage.setItem(STORAGE_KEY, '1');
      overlay.remove();
      STEPS.forEach(function (s) {
        var el = containerEl.querySelector(s.target);
        if (el) el.classList.remove('tour-highlight');
      });
    }

    prevBtn.addEventListener('click', function () { if (currentStep > 0) show(currentStep - 1); });
    nextBtn.addEventListener('click', function () {
      if (currentStep >= STEPS.length - 1) finish();
      else show(currentStep + 1);
    });
    closeBtn.addEventListener('click', finish);
    overlay.querySelector('.tour-backdrop').addEventListener('click', finish);

    show(0);
  }

  window.__leanctxTour = {
    start: function (containerEl) { runTour(containerEl); },
    shouldShow: function () { return !localStorage.getItem(STORAGE_KEY); },
    reset: function () { localStorage.removeItem(STORAGE_KEY); }
  };
})();
