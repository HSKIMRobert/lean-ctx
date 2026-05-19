/**
 * Scenario tests for cockpit-compression.js _collectFiles logic.
 * Run with: node rust/src/dashboard/static/tests/cockpit-compression.test.js
 */

// Minimal shim to extract _collectFiles logic
function collectFiles(ledger, events, graphFiles) {
  var seen = Object.create(null);
  var ctx = [];
  var allEvents = [];

  var evtList = Array.isArray(events) ? events : [];
  for (var j = evtList.length - 1; j >= 0; j--) {
    var ev = evtList[j];
    var kind = ev.kind || {};
    if (kind.type === 'ToolCall' && kind.path) {
      var sent = kind.tokens_compressed != null
        ? kind.tokens_compressed
        : (kind.tokens_original && kind.tokens_saved
          ? kind.tokens_original - kind.tokens_saved
          : 0);
      var row = {
        path: kind.path,
        mode: kind.mode || 'full',
        original: kind.tokens_original || 0,
        sent: sent,
        timestamp: ev.timestamp || null,
        tool: kind.tool || null,
      };
      allEvents.push(row);
      if (!seen[kind.path]) {
        seen[kind.path] = true;
        ctx.push(row);
      }
    }
  }

  if (ledger && Array.isArray(ledger.entries)) {
    for (var i = 0; i < ledger.entries.length; i++) {
      var e = ledger.entries[i];
      if (e.path && seen[e.path]) {
        var existing = ctx.find(function (c) { return c.path === e.path; });
        if (existing && existing.sent === 0 && e.sent_tokens > 0) {
          existing.sent = e.sent_tokens;
          existing.original = e.original_tokens || existing.original;
        }
      } else if (e.path && !seen[e.path]) {
        seen[e.path] = true;
        ctx.push({
          path: e.path,
          mode: e.active_view || e.mode || 'full',
          original: e.original_tokens || 0,
          sent: e.sent_tokens || 0,
          timestamp: null,
          tool: null,
        });
      }
    }
  }

  return { ctx, allEvents };
}

// --- Test Helpers ---
let passed = 0;
let failed = 0;

function assert(cond, msg) {
  if (!cond) {
    console.error(`  FAIL: ${msg}`);
    failed++;
  } else {
    console.log(`  PASS: ${msg}`);
    passed++;
  }
}

function assertEqual(actual, expected, msg) {
  if (actual !== expected) {
    console.error(`  FAIL: ${msg} — expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    failed++;
  } else {
    console.log(`  PASS: ${msg}`);
    passed++;
  }
}

// --- Scenarios ---

console.log('\n=== Scenario 1: ctx_tree event with tokens_saved but no tokens_compressed ===');
(function () {
  var events = [{
    id: 46,
    timestamp: '2026-05-19T16:12:09.515',
    kind: {
      type: 'ToolCall',
      tool: 'ctx_tree',
      tokens_original: 212,
      tokens_saved: 8,
      tokens_compressed: null,
      mode: null,
      path: '/workspace/project/src/tests'
    }
  }];
  var ledger = { entries: [{
    path: '/workspace/project/src/tests',
    original_tokens: 212,
    sent_tokens: 204,
    mode: 'full',
    active_view: 'full'
  }]};

  var result = collectFiles(ledger, events, null);
  assertEqual(result.ctx.length, 1, 'grouped has 1 entry');
  assertEqual(result.ctx[0].sent, 204, 'sent = 212 - 8 = 204 (derived from tokens_saved)');
  assertEqual(result.ctx[0].original, 212, 'original preserved');
  assertEqual(result.allEvents.length, 1, 'allEvents has 1 entry');
})();

console.log('\n=== Scenario 2: ctx_read event with tokens_compressed present ===');
(function () {
  var events = [{
    id: 10,
    timestamp: '2026-05-19T10:00:00',
    kind: {
      type: 'ToolCall',
      tool: 'ctx_read',
      tokens_original: 1000,
      tokens_saved: 850,
      tokens_compressed: 150,
      mode: 'map',
      path: '/workspace/src/main.rs'
    }
  }];

  var result = collectFiles(null, events, null);
  assertEqual(result.ctx[0].sent, 150, 'sent uses tokens_compressed directly when present');
  assertEqual(result.ctx[0].mode, 'map', 'mode preserved');
})();

console.log('\n=== Scenario 3: Ledger enriches event with sent=0 ===');
(function () {
  var events = [{
    id: 5,
    timestamp: '2026-05-19T09:00:00',
    kind: {
      type: 'ToolCall',
      tool: 'ctx_search',
      tokens_original: 500,
      tokens_saved: null,
      tokens_compressed: null,
      mode: null,
      path: '/workspace/lib/utils.py'
    }
  }];
  var ledger = { entries: [{
    path: '/workspace/lib/utils.py',
    original_tokens: 500,
    sent_tokens: 350,
    mode: 'aggressive',
    active_view: 'aggressive'
  }]};

  var result = collectFiles(ledger, events, null);
  assertEqual(result.ctx[0].sent, 350, 'ledger enriched sent from 0 to 350');
  assertEqual(result.ctx[0].original, 500, 'original stays 500');
})();

console.log('\n=== Scenario 4: Ledger entry for path not in events ===');
(function () {
  var events = [{
    id: 1,
    timestamp: '2026-05-19T08:00:00',
    kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 100, tokens_compressed: 50, path: '/a.rs' }
  }];
  var ledger = { entries: [
    { path: '/a.rs', original_tokens: 100, sent_tokens: 50, mode: 'full' },
    { path: '/b.rs', original_tokens: 800, sent_tokens: 200, mode: 'map', active_view: 'map' },
  ]};

  var result = collectFiles(ledger, events, null);
  assertEqual(result.ctx.length, 2, 'grouped has 2 entries (1 event + 1 ledger-only)');
  assertEqual(result.ctx[1].path, '/b.rs', 'ledger-only entry added');
  assertEqual(result.ctx[1].sent, 200, 'ledger-only entry has correct sent');
  assertEqual(result.ctx[1].mode, 'map', 'ledger-only entry uses active_view');
})();

console.log('\n=== Scenario 5: Dedup — multiple events for same path (grouped mode) ===');
(function () {
  var events = [
    { id: 1, timestamp: 'T1', kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 1000, tokens_compressed: 100, path: '/file.rs' } },
    { id: 2, timestamp: 'T2', kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 1000, tokens_compressed: 80, path: '/file.rs' } },
    { id: 3, timestamp: 'T3', kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 1000, tokens_compressed: 60, path: '/file.rs' } },
  ];

  var result = collectFiles(null, events, null);
  assertEqual(result.ctx.length, 1, 'grouped deduplicates to 1 entry');
  assertEqual(result.ctx[0].sent, 60, 'latest event wins (iterates reverse, so id=3 first)');
  assertEqual(result.allEvents.length, 3, 'allEvents has all 3 entries');
})();

console.log('\n=== Scenario 6: All Events mode shows every event individually ===');
(function () {
  var events = [
    { id: 1, timestamp: 'T1', kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 500, tokens_compressed: 100, path: '/x.rs', mode: 'map' } },
    { id: 2, timestamp: 'T2', kind: { type: 'ToolCall', tool: 'ctx_tree', tokens_original: 200, tokens_saved: 10, tokens_compressed: null, path: '/x.rs' } },
    { id: 3, timestamp: 'T3', kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 300, tokens_compressed: 30, path: '/y.rs', mode: 'signatures' } },
  ];

  var result = collectFiles(null, events, null);
  assertEqual(result.allEvents.length, 3, 'allEvents contains all 3 events');
  assertEqual(result.allEvents[0].path, '/y.rs', 'reverse order: last event first');
  assertEqual(result.allEvents[0].tool, 'ctx_read', 'tool name preserved');
  assertEqual(result.allEvents[1].path, '/x.rs', 'second event');
  assertEqual(result.allEvents[1].sent, 190, 'ctx_tree: 200 - 10 = 190');
  assertEqual(result.allEvents[1].tool, 'ctx_tree', 'tool name ctx_tree');
  assertEqual(result.allEvents[2].path, '/x.rs', 'third event (earliest)');
  assertEqual(result.allEvents[2].sent, 100, 'ctx_read tokens_compressed = 100');
})();

console.log('\n=== Scenario 7: Event with zero tokens_original and tokens_saved ===');
(function () {
  var events = [{
    id: 1,
    timestamp: 'T1',
    kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 0, tokens_saved: 0, tokens_compressed: null, path: '/empty.rs' }
  }];

  var result = collectFiles(null, events, null);
  assertEqual(result.ctx[0].sent, 0, 'sent is 0 when no data available');
  assertEqual(result.ctx[0].original, 0, 'original is 0');
})();

console.log('\n=== Scenario 8: Ledger does NOT overwrite valid event data ===');
(function () {
  var events = [{
    id: 1,
    timestamp: 'T1',
    kind: { type: 'ToolCall', tool: 'ctx_read', tokens_original: 1000, tokens_compressed: 150, path: '/valid.rs', mode: 'map' }
  }];
  var ledger = { entries: [{
    path: '/valid.rs',
    original_tokens: 1000,
    sent_tokens: 200,
    mode: 'full'
  }]};

  var result = collectFiles(ledger, events, null);
  assertEqual(result.ctx[0].sent, 150, 'event data preserved (sent > 0), ledger does not overwrite');
})();

console.log('\n=== Scenario 9: tagClass logic — sent > 0 gets "tg" ===');
(function () {
  var events = [{
    id: 46,
    timestamp: 'T1',
    kind: { type: 'ToolCall', tool: 'ctx_tree', tokens_original: 212, tokens_saved: 8, tokens_compressed: null, path: '/test' }
  }];

  var result = collectFiles(null, events, null);
  var f = result.ctx[0];
  var tagClass = f.sent > 0 ? 'tg' : 'ts';
  assertEqual(tagClass, 'tg', 'row with savings gets green tag class (tg)');
})();

// --- Summary ---
console.log(`\n${'='.repeat(50)}`);
console.log(`Results: ${passed} passed, ${failed} failed`);
if (failed > 0) {
  process.exit(1);
} else {
  console.log('All scenarios passed!');
}
