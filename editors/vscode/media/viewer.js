"use strict";

const vscode = acquireVsCodeApi();
const pagesElement = document.getElementById("pages");
const tailElement = document.getElementById("tail");
const errorElement = document.getElementById("error");

/** Assumed until the first frame says otherwise: US Letter. */
const DEFAULT_SIZE_BP = [612, 792];
/** A page drawn this far off the wanted scale is drawn again. */
const SCALE_TOLERANCE = 0.1;

/** Per page: its slot, size, overlay, the scale it was drawn at, and whether it is stale. */
const pages = [];
const requested = new Set();
let zoom = 1;
let viewTimer = null;
let lastView = "";
let highlightTimer = null;

function post(message) {
  vscode.postMessage(message);
}

function slotWidth() {
  return Math.max(64, pagesElement.clientWidth - 24) * zoom;
}

function scaleFor(page) {
  const width = page ? page.size[0] : DEFAULT_SIZE_BP[0];
  return (slotWidth() * window.devicePixelRatio) / width;
}

function layout() {
  const width = slotWidth();
  for (const page of pages) {
    page.slot.style.width = `${width}px`;
    page.slot.style.height = `${(width * page.size[1]) / page.size[0]}px`;
  }
}

function setPages(sizes, overlays) {
  const empty = document.getElementById("empty");
  if (empty && sizes.length > 0) empty.remove();
  while (pages.length < sizes.length) {
    const slot = document.createElement("section");
    slot.className = "page";
    slot.dataset.pageIndex = String(pages.length);
    const canvas = document.createElement("canvas");
    slot.appendChild(canvas);
    pagesElement.appendChild(slot);
    pages.push({ slot, canvas, size: DEFAULT_SIZE_BP, overlay: null, scale: 0, stale: true });
  }
  while (pages.length > sizes.length) pages.pop().slot.remove();
  pages.forEach((page, index) => {
    page.size = sizes[index];
    page.overlay = overlays[index] || null;
  });
  layout();
}

/** Show pixels laid out as width and height in little-endian `u32`, then RGBA rows. */
function draw(index, pixels, scale) {
  const page = pages[index];
  if (!page) return;
  const header = new DataView(pixels.buffer, pixels.byteOffset, 8);
  const width = header.getUint32(0, true);
  const height = header.getUint32(4, true);
  const rgba = new Uint8ClampedArray(pixels.buffer, pixels.byteOffset + 8, width * height * 4);
  page.canvas.width = width;
  page.canvas.height = height;
  page.canvas.getContext("2d").putImageData(new ImageData(rgba, width, height), 0, 0);
  page.scale = scale;
  page.stale = false;
}

function visibleRange() {
  const root = pagesElement.getBoundingClientRect();
  let first = -1;
  let last = -1;
  pages.forEach((page, index) => {
    const rect = page.slot.getBoundingClientRect();
    if (rect.bottom >= root.top && rect.top <= root.bottom) {
      if (first < 0) first = index;
      last = index;
    }
  });
  return first < 0 ? [0, 0] : [first, last];
}

/** Report the viewport, and ask for the pages near it that are stale or blurry. */
function viewChanged() {
  clearTimeout(viewTimer);
  viewTimer = setTimeout(() => {
    const [first, last] = visibleRange();
    const scale = scaleFor(pages[first]);
    const key = `${first}:${last}:${scale.toFixed(3)}`;
    if (key !== lastView) {
      lastView = key;
      post({ type: "view", viewport: [first, last], scale });
    }
    const wanted = [];
    for (let index = Math.max(0, first - 1); index <= Math.min(pages.length - 1, last + 1); index += 1) {
      const page = pages[index];
      const blurry = Math.abs(page.scale - scale) > SCALE_TOLERANCE * scale;
      if ((page.stale || blurry) && !requested.has(index)) wanted.push(index);
    }
    if (wanted.length > 0) {
      wanted.forEach((index) => requested.add(index));
      post({ type: "raster", pages: wanted, scale });
    }
  }, 60);
}

function compiled(report, scale) {
  if (!report.published) return;
  setPages(report.pageSizes, report.pageOverlays);
  for (const index of report.changed) if (pages[index]) pages[index].stale = true;
  for (const raster of report.rasters) draw(raster.index, raster.data, scale);
  tailElement.hidden = !report.tailPending;
  errorElement.hidden = true;
  viewChanged();
}

function showForwardSearch(rect) {
  document.querySelectorAll(".sync-highlight").forEach((node) => node.remove());
  clearTimeout(highlightTimer);
  const page = rect && pages[rect.pageIndex];
  if (!page) return;
  const [width, height] = page.size;
  const marker = document.createElement("div");
  marker.className = "sync-highlight";
  marker.style.left = `${(100 * rect.hBp) / width}%`;
  marker.style.top = `${(100 * (rect.vBp - rect.heightBp)) / height}%`;
  marker.style.width = `${(100 * Math.max(0, rect.widthBp)) / width}%`;
  marker.style.height = `${(100 * Math.max(0, rect.heightBp + rect.depthBp)) / height}%`;
  page.slot.appendChild(marker);
  page.slot.scrollIntoView({ behavior: "smooth", block: "center" });
  highlightTimer = setTimeout(() => marker.remove(), 1400);
}

function scrollToDestination(name) {
  for (const page of pages) {
    const found = page.overlay && page.overlay.destinations.find((destination) => destination.name === name);
    if (!found) continue;
    const top = (found.yBp / page.size[1]) * page.slot.clientHeight;
    pagesElement.scrollTo({ top: page.slot.offsetTop + top - 24, behavior: "smooth" });
    return;
  }
}

window.addEventListener("message", (message) => {
  const payload = message.data;
  if (!payload) return;
  if (payload.type === "compiled") {
    compiled(payload.report, payload.scale);
  } else if (payload.type === "rastered") {
    for (const raster of payload.pages) {
      requested.delete(raster.index);
      draw(raster.index, raster.data, payload.scale);
    }
  } else if (payload.type === "raster_failed") {
    payload.pages.forEach((index) => requested.delete(index));
  } else if (payload.type === "error") {
    errorElement.textContent = payload.message;
    errorElement.hidden = false;
  } else if (payload.type === "located") {
    showForwardSearch(payload.rect);
  }
});

pagesElement.addEventListener("click", (event) => {
  const slot = event.target.closest && event.target.closest(".page");
  if (!slot) return;
  const index = Number(slot.dataset.pageIndex);
  const page = pages[index];
  const bounds = slot.getBoundingClientRect();
  if (!page || bounds.width <= 0) return;
  const x = ((event.clientX - bounds.left) * page.size[0]) / bounds.width;
  const y = ((event.clientY - bounds.top) * page.size[1]) / bounds.height;
  if (event.ctrlKey || event.metaKey) {
    event.preventDefault();
    post({ type: "backward_search", pageIndex: index, hBp: x, vBp: y });
    return;
  }
  const link = page.overlay && page.overlay.links.find((candidate) =>
    x >= candidate.left && x <= candidate.right && y >= candidate.top && y <= candidate.bottom);
  if (!link) return;
  if (link.destination) scrollToDestination(link.destination);
  else if (link.uri) post({ type: "open_uri", uri: link.uri });
});

function setZoom(next) {
  zoom = next;
  layout();
  viewChanged();
}

document.getElementById("zoom-in").addEventListener("click", () => setZoom(Math.min(4, zoom * 1.2)));
document.getElementById("zoom-out").addEventListener("click", () => setZoom(Math.max(0.25, zoom / 1.2)));
document.getElementById("zoom-reset").addEventListener("click", () => setZoom(1));
new ResizeObserver(() => {
  layout();
  viewChanged();
}).observe(pagesElement);
pagesElement.addEventListener("scroll", viewChanged, { passive: true });

post({ type: "ready" });
viewChanged();
