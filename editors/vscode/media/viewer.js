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
let highlightTimer = null;

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
  // A single rAF callback fires *before* the frame containing our DOM
  // mutations is painted; the second one runs after that frame presented.
  await new Promise(requestAnimationFrame);
  await new Promise(requestAnimationFrame);
  post({
    type: "painted",
    frame_id: reduced.acknowledgement,
    paint_ms: Math.max(0, performance.now() - started)
  });
}

function showForwardSearch(result) {
  document.querySelectorAll(".sync-highlight").forEach((node) => node.remove());
  clearTimeout(highlightTimer);
  if (!result || !Number.isSafeInteger(result.page_index) || result.page_index < 0 ||
      result.page_index >= slots.length) return;
  const values = [result.h_bp, result.v_bp, result.width_bp, result.height_bp, result.depth_bp];
  if (!values.every(Number.isFinite)) return;
  const index = result.page_index;
  if (!state.pages[index].displayedSvg && state.pages[index].pendingSvg) {
    const indices = [...new Set([...state.prefetch, index])].sort((a, b) => a - b);
    const reduced = reducer.setPrefetch(state, indices);
    state = reduced.state;
    for (const installed of reduced.installed) renderPage(installed);
  }
  const size = state.pageSizes[index];
  if (!size) return;
  const marker = document.createElement("div");
  marker.className = "sync-highlight";
  marker.style.left = `${100 * result.h_bp / size.width_bp}%`;
  marker.style.top = `${100 * (result.v_bp - result.height_bp) / size.height_bp}%`;
  marker.style.width = `${100 * Math.max(0, result.width_bp) / size.width_bp}%`;
  marker.style.height = `${100 * Math.max(0, result.height_bp + result.depth_bp) / size.height_bp}%`;
  slots[index].appendChild(marker);
  slots[index].scrollIntoView({ behavior: "smooth", block: "center" });
  highlightTimer = setTimeout(() => marker.remove(), 1400);
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
  } else if (payload && payload.type === "forward_search") {
    showForwardSearch(payload.result);
  }
});

pagesElement.addEventListener("click", (event) => {
  if (!event.ctrlKey && !event.metaKey) return;
  const page = event.target.closest && event.target.closest(".page");
  if (!page) return;
  const pageIndex = Number(page.dataset.pageIndex);
  const size = state.pageSizes[pageIndex];
  const bounds = page.getBoundingClientRect();
  if (!size || bounds.width <= 0 || bounds.height <= 0) return;
  event.preventDefault();
  post({
    type: "backward_search",
    page_index: pageIndex,
    h_bp: Math.max(0, (event.clientX - bounds.left) * size.width_bp / bounds.width),
    v_bp: Math.max(0, (event.clientY - bounds.top) * size.height_bp / bounds.height)
  });
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
