"use strict";

const vscode = acquireVsCodeApi();
const pagesElement = document.getElementById("pages");
const progressElement = document.getElementById("progress");
const bannerElement = document.getElementById("banner");
const counterElement = document.getElementById("counter");
const emptyElement = document.getElementById("empty");

/** Assumed until the first frame says otherwise: US Letter. */
const DEFAULT_SIZE_BP = [612, 792];
/** A page drawn this far off the wanted scale is drawn again. */
const SCALE_TOLERANCE = 0.1;
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 5;
const ZOOM_STEP = 1.2;
const PAGE_GAP_PX = 16;

/** Per page: its slot, size, overlay, the scale it was drawn at, and whether it is stale. */
const pages = [];
const requested = new Set();
const saved = vscode.getState() || {};
let zoom = saved.zoom || 1;
let restoreTo = saved.position || null;
let viewTimer = null;
let lastView = "";
let highlightTimer = null;
let counterTimer = null;

function post(message) {
  vscode.postMessage(message);
}

function fitWidth() {
  return Math.max(64, pagesElement.clientWidth - 2 * PAGE_GAP_PX);
}

function slotWidth() {
  return fitWidth() * zoom;
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

/** Where the viewport is, as a page and a fraction into it, so it survives relayout. */
function position() {
  const top = pagesElement.scrollTop;
  const page = pages.find((candidate) => candidate.slot.offsetTop + candidate.slot.offsetHeight > top);
  if (!page) return null;
  return {
    page: pages.indexOf(page),
    fraction: (top - page.slot.offsetTop) / page.slot.offsetHeight,
    left: pagesElement.scrollWidth > pagesElement.clientWidth
      ? pagesElement.scrollLeft / pagesElement.scrollWidth
      : 0
  };
}

function restore(at) {
  const page = at && pages[at.page];
  if (!page) return false;
  pagesElement.scrollTop = page.slot.offsetTop + at.fraction * page.slot.offsetHeight;
  pagesElement.scrollLeft = at.left * pagesElement.scrollWidth;
  return true;
}

function remember() {
  vscode.setState({ zoom, position: position() });
}

function setPages(sizes, overlays) {
  const at = position();
  emptyElement.hidden = sizes.length > 0;
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
  if (restoreTo && restore(restoreTo)) restoreTo = null;
  else restore(at);
}

/**
 * Show pixels laid out as width and height in little-endian `u32`, then RGBA
 * rows. With a `region` they patch the page as it was last drawn at `scale`.
 */
function draw(index, pixels, scale, region) {
  const page = pages[index];
  if (!page) return;
  if (region && page.scale !== scale) {
    // The base this patch was made against is gone, so the whole page is needed.
    page.stale = true;
    return;
  }
  const header = new DataView(pixels.buffer, pixels.byteOffset, 8);
  const width = header.getUint32(0, true);
  const height = header.getUint32(4, true);
  if (width > 0 && height > 0) {
    const rgba = new Uint8ClampedArray(pixels.buffer, pixels.byteOffset + 8, width * height * 4);
    if (!region) {
      page.canvas.width = width;
      page.canvas.height = height;
    }
    const at = region || { x: 0, y: 0 };
    page.canvas.getContext("2d").putImageData(new ImageData(rgba, width, height), at.x, at.y);
  }
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
function viewChanged(delay = 60) {
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
    remember();
  }, delay);
}

function showCounter() {
  if (pages.length === 0) return;
  const [first] = visibleRange();
  counterElement.textContent = `${first + 1} / ${pages.length}`;
  counterElement.classList.add("visible");
  clearTimeout(counterTimer);
  counterTimer = setTimeout(() => counterElement.classList.remove("visible"), 900);
}

function compiled(report, scale) {
  if (!report.published) return;
  setPages(report.pageSizes, report.pageOverlays);
  for (const index of report.changed) if (pages[index]) pages[index].stale = true;
  for (const raster of report.rasters) draw(raster.index, raster.data, scale, raster.region);
  bannerElement.hidden = true;
  viewChanged();
}

/** Zoom by `factor`, keeping the point under (`clientX`, `clientY`) where it is. */
function zoomBy(factor, clientX, clientY) {
  const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom * factor));
  if (next === zoom) return;
  const bounds = pagesElement.getBoundingClientRect();
  const x = (clientX ?? bounds.left + bounds.width / 2) - bounds.left;
  const y = (clientY ?? bounds.top + bounds.height / 2) - bounds.top;
  const contentX = pagesElement.scrollLeft + x;
  const contentY = pagesElement.scrollTop + y;
  const ratio = next / zoom;
  zoom = next;
  layout();
  pagesElement.scrollLeft = contentX * ratio - x;
  pagesElement.scrollTop = contentY * ratio - y;
  viewChanged(150);
}

function setZoom(next) {
  zoomBy(next / zoom);
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
  marker.scrollIntoView({ behavior: "smooth", block: "center", inline: "nearest" });
  highlightTimer = setTimeout(() => marker.remove(), 1400);
}

function scrollToDestination(name) {
  for (const page of pages) {
    const found = page.overlay && page.overlay.destinations.find((destination) => destination.name === name);
    if (!found) continue;
    const top = (found.yBp / page.size[1]) * page.slot.clientHeight;
    pagesElement.scrollTo({ top: page.slot.offsetTop + top - PAGE_GAP_PX, behavior: "smooth" });
    return;
  }
}

/** The page under a pointer event and the point on it in bp. */
function pointOnPage(event) {
  const slot = event.target.closest && event.target.closest(".page");
  if (!slot) return null;
  const index = Number(slot.dataset.pageIndex);
  const page = pages[index];
  const bounds = slot.getBoundingClientRect();
  if (!page || bounds.width <= 0) return null;
  return {
    index,
    page,
    x: ((event.clientX - bounds.left) * page.size[0]) / bounds.width,
    y: ((event.clientY - bounds.top) * page.size[1]) / bounds.height
  };
}

function linkAt(point) {
  return point && point.page.overlay && point.page.overlay.links.find((link) =>
    point.x >= link.left && point.x <= link.right && point.y >= link.top && point.y <= link.bottom);
}

window.addEventListener("message", (message) => {
  const payload = message.data;
  if (!payload) return;
  if (payload.type === "compiled") {
    compiled(payload.report, payload.scale);
  } else if (payload.type === "rastered") {
    for (const raster of payload.pages) {
      requested.delete(raster.index);
      draw(raster.index, raster.data, payload.scale, raster.region);
    }
    viewChanged();
  } else if (payload.type === "raster_failed") {
    payload.pages.forEach((index) => requested.delete(index));
  } else if (payload.type === "busy") {
    progressElement.classList.toggle("active", payload.running);
  } else if (payload.type === "error") {
    bannerElement.textContent = payload.message;
    bannerElement.hidden = false;
  } else if (payload.type === "located") {
    showForwardSearch(payload.rect);
  } else if (payload.type === "zoom") {
    if (payload.action === "in") zoomBy(ZOOM_STEP);
    else if (payload.action === "out") zoomBy(1 / ZOOM_STEP);
    else setZoom(1);
  }
});

pagesElement.addEventListener("click", (event) => {
  const point = pointOnPage(event);
  if (!point) return;
  if (event.ctrlKey || event.metaKey) {
    event.preventDefault();
    post({ type: "backward_search", pageIndex: point.index, hBp: point.x, vBp: point.y });
    return;
  }
  const link = linkAt(point);
  if (!link) return;
  if (link.destination) scrollToDestination(link.destination);
  else if (link.uri) post({ type: "open_uri", uri: link.uri });
});

pagesElement.addEventListener("mousemove", (event) => {
  pagesElement.classList.toggle("over-link", Boolean(linkAt(pointOnPage(event))));
});

// A trackpad pinch arrives as a wheel event with ctrlKey set.
pagesElement.addEventListener("wheel", (event) => {
  if (!event.ctrlKey && !event.metaKey) return;
  event.preventDefault();
  const pixels = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * 16 : event.deltaY;
  zoomBy(Math.exp(-pixels / 300), event.clientX, event.clientY);
}, { passive: false });

window.addEventListener("keydown", (event) => {
  if (!event.ctrlKey && !event.metaKey) return;
  if (event.key === "=" || event.key === "+") zoomBy(ZOOM_STEP);
  else if (event.key === "-") zoomBy(1 / ZOOM_STEP);
  else if (event.key === "0") setZoom(1);
  else return;
  event.preventDefault();
});

new ResizeObserver(() => {
  const at = position();
  layout();
  restore(at);
  viewChanged();
}).observe(pagesElement);
pagesElement.addEventListener("scroll", () => {
  showCounter();
  viewChanged();
}, { passive: true });

post({ type: "ready" });
viewChanged();
