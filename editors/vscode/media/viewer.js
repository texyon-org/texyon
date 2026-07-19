"use strict";

const vscode = acquireVsCodeApi();
const reducer = globalThis.TectonicViewerState;
const pagesElement = document.getElementById("pages");
const tailElement = document.getElementById("tail");
const errorElement = document.getElementById("error");
const slots = [];
const prefetch = new Set();
let state = reducer.initialState();
let queue = Promise.resolve();
let zoom = 1;
let viewportTimer = null;
let lastViewport = null;

function post(message) {
  vscode.postMessage(message);
}

function targetWidth() {
  return Math.max(64, pagesElement.clientWidth - 24) * zoom;
}

function resizeSlots() {
  const width = targetWidth();
  for (let index = 0; index < slots.length; index += 1) {
    const size = state.pageSizes[index];
    slots[index].style.width = `${width}px`;
    if (!slots[index].firstElementChild && size) {
      slots[index].style.minHeight = `${width * size.height_bp / size.width_bp}px`;
    }
  }
}

function ensureSlots() {
  const empty = document.getElementById("empty");
  if (empty && state.pages.length > 0) empty.remove();
  while (slots.length < state.pages.length) {
    const page = document.createElement("section");
    page.className = "page";
    page.dataset.pageIndex = String(slots.length);
    pagesElement.appendChild(page);
    slots.push(page);
    observer.observe(page);
  }
  while (slots.length > state.pages.length) {
    const page = slots.pop();
    observer.unobserve(page);
    prefetch.delete(Number(page.dataset.pageIndex));
    page.remove();
  }
  resizeSlots();
}

function renderPage(index) {
  const markup = state.pages[index] && state.pages[index].displayedSvg;
  if (!markup) return;
  const parsed = new DOMParser().parseFromString(markup, "image/svg+xml");
  const node = document.importNode(parsed.documentElement, true);
  slots[index].replaceChildren(node);
  slots[index].style.minHeight = "";
}

function trueVisibleRange() {
  if (slots.length === 0) return null;
  const root = pagesElement.getBoundingClientRect();
  let first = Infinity;
  let last = -1;
  for (let index = 0; index < slots.length; index += 1) {
    const rect = slots[index].getBoundingClientRect();
    if (rect.bottom >= root.top && rect.top <= root.bottom) {
      first = Math.min(first, index);
      last = Math.max(last, index);
    }
  }
  return last < 0 ? null : { first, last };
}

function updateVisibility() {
  const visible = trueVisibleRange();
  if (!visible) return;
  state = reducer.setVisible(state, visible.first, visible.last);
  clearTimeout(viewportTimer);
  viewportTimer = setTimeout(() => {
    const key = `${visible.first}:${visible.last}`;
    if (key !== lastViewport) {
      lastViewport = key;
      post({ type: "viewport", first: visible.first, last: visible.last });
    }
  }, 100);
}

const observer = new IntersectionObserver((entries) => {
  for (const entry of entries) {
    const index = Number(entry.target.dataset.pageIndex);
    if (entry.isIntersecting) prefetch.add(index);
    else prefetch.delete(index);
  }
  const reduced = reducer.setPrefetch(state, [...prefetch].sort((a, b) => a - b));
  state = reduced.state;
  for (const index of reduced.installed) renderPage(index);
  updateVisibility();
}, { root: pagesElement, rootMargin: "100% 0px 100% 0px" });

function sanitize(svg, prefix) {
  return reducer.sanitizeSvg(svg, prefix, DOMParser, XMLSerializer);
}

async function applyUpdate(event) {
  const started = performance.now();
  const previous = state;
  const reduced = reducer.applyUpdate(state, event, sanitize);
  state = reduced.state;
  ensureSlots();
  for (let index = 0; index < state.pages.length; index += 1) {
    if (state.pages[index].displayedSvg !== (previous.pages[index] && previous.pages[index].displayedSvg)) {
      renderPage(index);
    }
  }
  tailElement.hidden = !state.tailPending;
  errorElement.hidden = true;
  updateVisibility();
  await new Promise(requestAnimationFrame);
  post({
    type: "painted",
    frame_id: reduced.acknowledgement,
    paint_ms: Math.max(0, performance.now() - started)
  });
}

window.addEventListener("message", (message) => {
  const payload = message.data;
  if (payload && payload.type === "update") {
    queue = queue.then(() => applyUpdate(payload.event)).catch((error) => {
      errorElement.textContent = `Rejected preview frame: ${error.message}`;
      errorElement.hidden = false;
      post({ type: "viewer_error", message: error.message });
    });
  } else if (payload && payload.type === "session_error") {
    errorElement.textContent = payload.message;
    errorElement.hidden = false;
  }
});

document.getElementById("zoom-in").addEventListener("click", () => {
  zoom = Math.min(4, zoom * 1.2);
  resizeSlots();
  updateVisibility();
});
document.getElementById("zoom-out").addEventListener("click", () => {
  zoom = Math.max(0.25, zoom / 1.2);
  resizeSlots();
  updateVisibility();
});
document.getElementById("zoom-reset").addEventListener("click", () => {
  zoom = 1;
  resizeSlots();
  updateVisibility();
});
new ResizeObserver(() => {
  resizeSlots();
  updateVisibility();
}).observe(pagesElement);
pagesElement.addEventListener("scroll", updateVisibility, { passive: true });

post({ type: "ready" });
