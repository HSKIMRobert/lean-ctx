#!/usr/bin/env node
/**
 * Regression test for targeted dashboard refreshes.
 * Run with: node rust/src/dashboard/static/tests/dashboard-refresh.test.js
 */

const fs = require('node:fs');
const path = require('node:path');

const indexPath = path.join(__dirname, '..', 'index.html');
const html = fs.readFileSync(indexPath, 'utf8');
const doctorPath = path.join(__dirname, '..', 'lib', 'doctor.js');
const doctorSource = fs.readFileSync(doctorPath, 'utf8');

let scriptStart = html.indexOf('<script');
while (scriptStart >= 0) {
  const sourceStart = html.indexOf('>', scriptStart) + 1;
  const sourceEnd = html.indexOf('</script>', sourceStart);
  if (sourceStart === 0 || sourceEnd < 0) {
    throw new Error('unterminated inline script in dashboard shell');
  }
  const source = html.slice(sourceStart, sourceEnd).trim();
  if (source) Function(source);
  scriptStart = html.indexOf('<script', sourceEnd + '</script>'.length);
}
const helperMarker = 'function refreshActiveView()';
const routerInitMarker = 'if (window.LctxRouter && window.LctxRouter.init)';
const helperStart = html.indexOf(helperMarker);
const routerInit = html.indexOf(routerInitMarker, helperStart);

if (helperStart < 0 || routerInit < 0) {
  throw new Error('targeted refresh coordinator is missing');
}

const coordinator = html.slice(helperStart, routerInit);
const refreshCalls = coordinator.match(/refreshActiveView\(\);/g) || [];

if (!coordinator.includes('router.navigateTo(router.getActiveViewId())')) {
  throw new Error('refresh does not target the active view loader');
}
if (refreshCalls.length !== 3) {
  throw new Error(`expected 3 targeted refresh paths, found ${refreshCalls.length}`);
}
if (!coordinator.includes('hasPendingUpdate')) {
  throw new Error('visibility observer ignores pending updates');
}
if (coordinator.includes("CustomEvent('lctx:refresh')")) {
  throw new Error('refresh coordinator still broadcasts to every panel');
}
if (!doctorSource.includes('router.navigateTo(router.getActiveViewId())')) {
  throw new Error('Doctor fix does not target the active view loader');
}
if (doctorSource.includes("CustomEvent('lctx:refresh')")) {
  throw new Error('Doctor fix still broadcasts to every panel');
}

console.log('PASS: dashboard refresh targets only the active view');
