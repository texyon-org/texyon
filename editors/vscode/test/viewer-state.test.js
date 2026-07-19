"use strict";

const assert = require("node:assert/strict");
const test = require("node:test");
const viewer = require("../media/viewer-state");

const sanitize = (svg, prefix) => {
  viewer.preflightSvg(svg);
  return `${prefix}:${svg}`;
};

function update(overrides = {}) {
  return {
    type: "update",
    frame_id: "1",
    source_revision: "0",
    applied_request_seq: "0",
    phase: "foreground",
    outcome: "complete",
    page_count: 1,
    changed: [0],
    removed: [],
    pages_svg: [{ page_index: 0, svg: "<svg/>" }],
    page_sizes: [{ width_bp: 100, height_bp: 200 }],
    unsupported_pages: [],
    metrics: {
      engine_ms: 1,
      analysis_ms: 1,
      render_ms: 1,
      frame_bytes: 1,
      checkpoint_rungs: 1,
      checkpoint_bytes: 1,
      shipouts: 1,
      reused_prefix: false,
      pages_fingerprinted: 1,
      pages_rendered: 1,
      unsupported_pages: 0
    },
    tail_pending: false,
    ...overrides
  };
}

test("rejects stale frames and decreasing source revisions atomically", () => {
  const first = viewer.applyUpdate(viewer.initialState(), update(), sanitize).state;
  assert.throws(() => viewer.applyUpdate(first, update(), sanitize), /strictly increase/);
  assert.throws(() => viewer.applyUpdate(first, update({ frame_id: "2", source_revision: "00" }), sanitize), /canonical/);
  const later = viewer.applyUpdate(first, update({ frame_id: "2", source_revision: "2" }), sanitize).state;
  assert.throws(() => viewer.applyUpdate(later, update({ frame_id: "3", source_revision: "1" }), sanitize), /decreased/);
  assert.equal(later.lastFrameId, "2");
});

test("accepts a same-revision tail settle under a newer frame", () => {
  const first = viewer.applyUpdate(viewer.initialState(), update(), sanitize).state;
  const settled = viewer.applyUpdate(first, update({
    frame_id: "2",
    phase: "tail_settle",
    changed: [],
    pages_svg: []
  }), sanitize);
  assert.equal(settled.state.sourceRevision, "0");
  assert.equal(settled.state.phase, "tail_settle");
  assert.equal(settled.acknowledgement, "2");
});

test("keeps only the newest offscreen SVG and installs the plus-or-minus-one band", () => {
  const threePages = update({
    page_count: 3,
    changed: [0, 1, 2],
    pages_svg: [0, 1, 2].map((page_index) => ({ page_index, svg: `<svg id="p${page_index}"/>` })),
    page_sizes: [0, 1, 2].map(() => ({ width_bp: 100, height_bp: 200 }))
  });
  let state = viewer.applyUpdate(viewer.initialState(), threePages, sanitize).state;
  assert.ok(state.pages.every((page) => page.pendingSvg !== null));
  state = viewer.applyUpdate(state, update({
    frame_id: "2",
    source_revision: "1",
    page_count: 3,
    changed: [2],
    pages_svg: [{ page_index: 2, svg: "<svg id=\"new\"/>" }],
    page_sizes: [0, 1, 2].map(() => ({ width_bp: 100, height_bp: 200 }))
  }), sanitize).state;
  assert.match(state.pages[2].pendingSvg, /new/);
  state = viewer.setVisible(state, 0, 0);
  assert.ok(state.pages.every((page) => page.pendingSvg !== null));
  const prefetched = viewer.setPrefetch(state, [0, 1]);
  assert.deepEqual(prefetched.installed, [0, 1]);
  assert.equal(prefetched.state.pages[2].displayedSvg, null);
  assert.deepEqual(prefetched.state.visible, { first: 0, last: 0 });
  assert.deepEqual(prefetched.state.prefetch, [0, 1]);
});

test("metadata-only updates grow and shrink persistent slots coherently", () => {
  let state = viewer.applyUpdate(viewer.initialState(), update(), sanitize).state;
  state = viewer.applyUpdate(state, update({
    frame_id: "2",
    page_count: 2,
    changed: [1],
    pages_svg: [{ page_index: 1, svg: "<svg/>" }],
    page_sizes: [{ width_bp: 100, height_bp: 200 }, { width_bp: 200, height_bp: 100 }],
    tail_pending: true
  }), sanitize).state;
  assert.equal(state.pages.length, 2);
  const shrunk = viewer.applyUpdate(state, update({
    frame_id: "3",
    phase: "resync",
    changed: [],
    removed: [1],
    pages_svg: [],
    tail_pending: false
  }), sanitize).state;
  assert.equal(shrunk.pages.length, 1);
  assert.equal(shrunk.tailPending, false);
});

test("unsupported renderer telemetry advances on metadata-only frames", () => {
  let state = viewer.applyUpdate(viewer.initialState(), update({
    unsupported_pages: [{ page_index: 0, operations: { "missing-image-resource": 1 } }]
  }), sanitize).state;
  assert.deepEqual(state.unsupportedPages[0].operations, { "missing-image-resource": 1 });
  state = viewer.applyUpdate(state, update({
    frame_id: "2",
    changed: [],
    pages_svg: [],
    unsupported_pages: []
  }), sanitize).state;
  assert.deepEqual(state.unsupportedPages, []);
  assert.throws(() => viewer.applyUpdate(state, update({
    frame_id: "3",
    unsupported_pages: [{ page_index: 1, operations: { bad: 1 } }]
  }), sanitize), /unsupported page telemetry/);
});

test("rejects partial patches without changing retained state", () => {
  const state = viewer.applyUpdate(viewer.initialState(), update(), sanitize).state;
  assert.throws(() => viewer.applyUpdate(state, update({
    frame_id: "2",
    changed: [0],
    pages_svg: []
  }), sanitize), /disagree/);
  assert.equal(state.lastFrameId, "1");
  assert.equal(state.pages.length, 1);
});

test("counter strings remain exact beyond JavaScript's safe integer range", () => {
  const large = "9007199254740993";
  const state = viewer.applyUpdate(viewer.initialState(), update({ frame_id: large }), sanitize).state;
  const next = viewer.applyUpdate(state, update({ frame_id: "9007199254740994" }), sanitize).state;
  assert.equal(next.lastFrameId, "9007199254740994");
});

test("hostile SVG preflight rejects active content and external resources", () => {
  for (const svg of [
    "<svg><script>alert(1)</script></svg>",
    "<svg onload=\"alert(1)\"/>",
    "<svg><foreignObject/></svg>",
    "<svg><animate attributeName=\"opacity\"/></svg>",
    "<!DOCTYPE svg><svg/>",
    "<?xml-stylesheet href=\"x\"?><svg/>",
    "<svg><path style=\"fill:url(https://example.com/x)\"/></svg>",
    "<svg><a href=\"javascript:alert(1)\"/></svg>"
  ]) {
    assert.throws(() => viewer.preflightSvg(svg));
  }
  assert.doesNotThrow(() => viewer.preflightSvg("<svg><use href=\"#glyph\"/></svg>"));
  assert.equal(viewer.rewriteHref("#glyph", "f1_p0"), "#f1_p0__glyph");
  assert.equal(viewer.rewriteUrlFunctions("fill:url(#paint)", "f1_p1"), "fill:url(#f1_p1__paint)");
  assert.notEqual(viewer.rewriteHref("#glyph", "f1_p0"), viewer.rewriteHref("#glyph", "f1_p1"));
});
