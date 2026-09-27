import type { CompileResult, Link, Located, SyncRect } from "@tectonic/wasm";
import { useCallback, useEffect, useRef, useState } from "react";

import { measure } from "../perf";

/** Zoom steps, in the order the buttons walk through them. */
const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3] as const;

/** How long a forward search highlight stays, including its fade. */
const HIGHLIGHT_MS = 1500;

export interface ViewerProps {
  /** The latest compile, or null before the first one. */
  result: CompileResult | null;
  /** Called once the changed pages are on screen. */
  onPainted: () => void;
  /** Someone clicked a point on a page; find the source line that made it. */
  onBackwardSearch: (pageIndex: number, hBp: number, vBp: number) => void;
  /** Where a forward search found the cursor's line, and what to flash. */
  highlight: Located | null;
  /**
   * The visible page range moved, zero-based and inclusive.
   *
   * This is what lets a compile stop at the last page the user can see and
   * leave the rest for idle time, so it is reported on every scroll, zoom and
   * layout change.
   */
  onViewport: (first: number, last: number) => void;
}

/**
 * The rendered document.
 *
 * The engine hands back a *patch* — only the pages whose appearance changed —
 * so this keeps a DOM slot per page and replaces the contents of just those. A
 * one-word edit in a twenty-page document touches one slot; the other nineteen
 * are never re-parsed, never re-laid-out and never repainted. That is the whole
 * reason the preview stays cheap, and it is why this component manipulates DOM
 * directly instead of rendering pages as React children.
 */
export function Viewer({ result, onPainted, onBackwardSearch, highlight, onViewport }: ViewerProps) {
  const scroller = useRef<HTMLDivElement>(null);
  const container = useRef<HTMLDivElement>(null);
  const slots = useRef<HTMLElement[]>([]);
  const [zoom, setZoom] = useState(1);
  const [pageCount, setPageCount] = useState(0);

  // Read through a ref so the click handler installed on each page does not
  // have to be replaced every time the zoom changes.
  const scale = useRef(zoom);
  scale.current = zoom;

  const backward = useRef(onBackwardSearch);
  backward.current = onBackwardSearch;

  const reportViewport = useRef(onViewport);
  reportViewport.current = onViewport;

  /** The effect below also reruns on zoom; only a new frame is instrumented. */
  const lastApplied = useRef<CompileResult | null>(null);

  /**
   * The last result that actually published a frame.
   *
   * An aborted or cancelled run reports empty page fields that mean "keep what
   * you have"; applying them would wipe every page slot. The viewer keeps
   * showing this frame instead until a published one replaces it.
   */
  const lastPublished = useRef<CompileResult | null>(null);

  /** Say which pages currently intersect the scroller. */
  const measureViewport = useCallback(() => {
    const view = scroller.current;
    if (view === null || slots.current.length === 0) return;

    // Client rects rather than offsetTop, which depends on which ancestor
    // happens to be positioned.
    const box = view.getBoundingClientRect();
    let first: number | null = null;
    let last: number | null = null;
    for (const [index, slot] of slots.current.entries()) {
      const page = slot.getBoundingClientRect();
      if (page.bottom <= box.top) continue;
      if (page.top >= box.bottom) break;
      first ??= index;
      last = index;
    }
    if (first !== null && last !== null) reportViewport.current(first, last);
  }, []);

  useEffect(() => {
    const view = scroller.current;
    if (view === null) return;

    // One measurement per frame, however fast the scroll events come.
    let frame = 0;
    const onScroll = (): void => {
      if (frame !== 0) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        measureViewport();
      });
    };
    view.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      view.removeEventListener("scroll", onScroll);
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  }, [measureViewport]);

  /** Where on the page, in big points, a click landed. */
  const pointOf = useCallback((event: MouseEvent, slot: HTMLElement): [number, number] => {
    const box = slot.getBoundingClientRect();
    return [(event.clientX - box.left) / scale.current, (event.clientY - box.top) / scale.current];
  }, []);

  useEffect(() => {
    if (result !== null && result.published) lastPublished.current = result;
    const frame = lastPublished.current;
    const host = container.current;
    if (frame === null || host === null) return;

    const fresh = frame !== lastApplied.current;
    lastApplied.current = frame;
    if (fresh) {
      performance.mark("tex:apply-start");
      measure("tex:react-commit", "tex:frame-received", "tex:apply-start");
    }

    while (slots.current.length < frame.pageCount) {
      const slot = document.createElement("section");
      slot.className = "page";
      slot.addEventListener("dblclick", (event) => {
        const index = slots.current.indexOf(slot);
        if (index < 0) return;
        const [hBp, vBp] = pointOf(event, slot);
        backward.current(index, hBp, vBp);
      });
      host.appendChild(slot);
      slots.current.push(slot);
    }
    while (slots.current.length > frame.pageCount) {
      slots.current.pop()?.remove();
    }

    // Size every slot, including ones we were not sent, so the scroll height is
    // right before their pages arrive.
    frame.pageSizes.forEach(([width, height], index) => {
      const slot = slots.current[index];
      if (slot === undefined) return;
      slot.style.width = `${width * zoom}px`;
      slot.style.height = `${height * zoom}px`;
    });

    for (const page of frame.pages) {
      const slot = slots.current[page.index];
      if (slot === undefined) continue;

      const parseFrom = performance.now();
      const svg = parse(page.svg, page.index);
      if (svg === null) continue;
      const parsedAt = performance.now();

      svg.setAttribute("width", "100%");
      svg.setAttribute("height", "100%");
      slot.replaceChildren(svg);
      if (fresh) {
        performance.measure(`tex:parse-p${page.index}`, { start: parseFrom, end: parsedAt });
        performance.measure(`tex:dom-p${page.index}`, { start: parsedAt, end: performance.now() });
      }
    }
    if (fresh) {
      performance.mark("tex:pages-applied");
      measure("tex:pages", "tex:apply-start", "tex:pages-applied");
    }

    // Links are not drawn — they arrive beside the page as regions — so they go
    // on as a layer of anchors above the SVG. Every page's overlay is rebuilt,
    // because a page whose text did not move can still have had its link
    // renumbered by an edit above it, and because the zoom may have changed.
    frame.pageOverlays.forEach((overlay, index) => {
      const slot = slots.current[index];
      if (slot === undefined) return;

      slot.querySelector(".links")?.remove();
      if (overlay.links.length === 0) return;

      const layer = document.createElement("div");
      layer.className = "links";
      for (const link of overlay.links) {
        layer.appendChild(anchorFor(link, zoom, frame, slots.current, scroller.current));
      }
      slot.appendChild(layer);
    });

    if (fresh) {
      performance.mark("tex:overlays-applied");
      measure("tex:overlays", "tex:pages-applied", "tex:overlays-applied");
    }

    setPageCount(frame.pageCount);

    // The document may have grown, shrunk or been rescaled under a still
    // scrollbar, all of which change which pages are visible.
    measureViewport();

    if (frame.pages.length > 0) {
      // The rAF callback runs just before the frame that shows the new page is
      // rendered, and a task queued from it runs just after that frame is
      // presented. A second rAF here would wait a whole extra frame.
      requestAnimationFrame(() =>
        setTimeout(() => {
          if (fresh) {
            performance.mark("tex:painted");
            measure("tex:paint-wait", "tex:overlays-applied", "tex:painted");
          }
          onPainted();
        }, 0),
      );
    }
  }, [result, zoom, onPainted, pointOf, measureViewport]);

  // A forward search: bring the best box into view and flash the boxes, or the
  // page when there are none. Zoom is read through the ref so zooming does not
  // replay it.
  useEffect(() => {
    const frame = lastPublished.current;
    const view = scroller.current;
    if (highlight === null || frame === null || view === null) return;

    const marks: HTMLElement[] = [];
    const drawn = new Set<string>();
    for (const box of highlight.boxes) {
      const slot = slots.current[box.pageIndex];
      const size = frame.pageSizes[box.pageIndex];
      if (slot === undefined || size === undefined) continue;
      // Nested boxes often report the same rectangle, and stacked tints go opaque.
      const key = [box.pageIndex, box.hBp, box.vBp, box.widthBp, box.heightBp, box.depthBp].join();
      if (drawn.has(key)) continue;
      drawn.add(key);
      const mark = markFor(box, size);
      if (mark === null) continue;
      slot.appendChild(mark);
      marks.push(mark);
    }

    const first = highlight.rect;
    const slot = slots.current[first.pageIndex];
    if (slot !== undefined) {
      const zoom = scale.current;
      const top = slot.offsetTop + Math.max(first.vBp - first.heightBp, 0) * zoom;
      const bottom = slot.offsetTop + (first.vBp + first.depthBp) * zoom;
      const left = slot.offsetLeft + first.hBp * zoom;
      const margin = view.clientHeight / 6;
      const shown = top >= view.scrollTop + margin && bottom <= view.scrollTop + view.clientHeight - margin;
      const reachable = left >= view.scrollLeft && left <= view.scrollLeft + view.clientWidth;
      if (!shown || !reachable) {
        view.scrollTo({
          top: shown ? view.scrollTop : top - view.clientHeight / 3,
          left: reachable ? view.scrollLeft : left - 40,
          behavior: "smooth",
        });
      }
    }

    const page = marks.length === 0 ? slot : undefined;
    page?.classList.add("located");
    const clear = (): void => {
      marks.forEach((mark) => mark.remove());
      page?.classList.remove("located");
    };
    const timer = window.setTimeout(clear, HIGHLIGHT_MS);
    return () => {
      window.clearTimeout(timer);
      clear();
    };
  }, [highlight]);

  const step = (direction: 1 | -1): void => {
    const at = ZOOM_STEPS.indexOf(zoom as (typeof ZOOM_STEPS)[number]);
    const next = at < 0 ? 2 : Math.min(ZOOM_STEPS.length - 1, Math.max(0, at + direction));
    setZoom(ZOOM_STEPS[next] ?? 1);
  };

  return (
    <div className="viewer">
      <div className="viewer-bar">
        <button onClick={() => step(-1)} disabled={zoom <= ZOOM_STEPS[0]} title="Zoom out">
          −
        </button>
        <button className="zoom" onClick={() => setZoom(1)} title="Reset zoom">
          {Math.round(zoom * 100)}%
        </button>
        <button
          onClick={() => step(1)}
          disabled={zoom >= ZOOM_STEPS[ZOOM_STEPS.length - 1]}
          title="Zoom in"
        >
          +
        </button>
        <span className="viewer-hint">Double-click a page to jump to its source</span>
        {pageCount > 0 && (
          <span className="viewer-pages">
            {pageCount} page{pageCount === 1 ? "" : "s"}
          </span>
        )}
      </div>

      <div className="viewer-scroll" ref={scroller}>
        {pageCount === 0 && <div className="viewer-empty">Waiting for the first compile…</div>}
        <div className="viewer-pages-host" ref={container} />
      </div>
    </div>
  );
}

/**
 * A fading highlight over one forward search box, or null when none of it is on
 * the page.
 *
 * Clipped to the page, since a box can reach past it (a beamer headline, a
 * negative offset). Placed in percent of the page, so it follows the zoom
 * without being redrawn.
 */
function markFor(box: SyncRect, [width, height]: [number, number]): HTMLElement | null {
  const clamp = (value: number, max: number): number => Math.min(Math.max(value, 0), max);
  const top = box.vBp - box.heightBp;
  const left = clamp(box.hBp, width);
  const right = clamp(box.hBp + Math.max(box.widthBp, 1), width);
  const upper = clamp(top, height);
  const lower = clamp(top + Math.max(box.heightBp + box.depthBp, 1), height);
  if (right <= left || lower <= upper) return null;

  const mark = document.createElement("div");
  mark.className = "sync-highlight";
  mark.style.left = `${(left / width) * 100}%`;
  mark.style.top = `${(upper / height) * 100}%`;
  mark.style.width = `${((right - left) / width) * 100}%`;
  mark.style.height = `${((lower - upper) / height) * 100}%`;
  mark.style.animationDuration = `${HIGHLIGHT_MS}ms`;
  return mark;
}

/** One clickable region, positioned over the page it belongs to. */
function anchorFor(
  link: Link,
  zoom: number,
  result: CompileResult,
  slots: readonly HTMLElement[],
  scroller: HTMLElement | null,
): HTMLAnchorElement {
  const anchor = document.createElement("a");
  anchor.style.left = `${link.left * zoom}px`;
  anchor.style.top = `${link.top * zoom}px`;
  anchor.style.width = `${(link.right - link.left) * zoom}px`;
  anchor.style.height = `${(link.bottom - link.top) * zoom}px`;

  if (link.uri !== null) {
    if (!isSafeUri(link.uri)) return anchor;
    anchor.href = link.uri;
    anchor.target = "_blank";
    // The document is untrusted input; an opened tab must not reach back.
    anchor.rel = "noreferrer noopener";
    anchor.title = link.uri;
    return anchor;
  }

  const destination = link.destination;
  if (destination !== null) {
    anchor.href = `#${destination}`;
    anchor.title = destination;
    anchor.onclick = (event) => {
      event.preventDefault();
      jumpTo(destination, result, slots, zoom, scroller);
    };
  }

  return anchor;
}

const SAFE_SCHEMES = new Set(["http:", "https:", "mailto:"]);

function isSafeUri(uri: string): boolean {
  try {
    return SAFE_SCHEMES.has(new URL(uri).protocol);
  } catch {
    return false;
  }
}

/**
 * Scroll to a named destination, wherever in the document it landed.
 *
 * Destinations are per page, so following one means first finding which page
 * declared the name.
 */
function jumpTo(
  name: string,
  result: CompileResult,
  slots: readonly HTMLElement[],
  zoom: number,
  scroller: HTMLElement | null,
): void {
  for (const [index, overlay] of result.pageOverlays.entries()) {
    const target = overlay.destinations.find((destination) => destination.name === name);
    if (target === undefined) continue;

    const slot = slots[index];
    if (slot === undefined || scroller === null) return;

    scroller.scrollTo({ top: slot.offsetTop + target.yBp * zoom - 40, behavior: "smooth" });
    return;
  }
}

/**
 * Parse one page's SVG into a node, with its internal ids made unique.
 *
 * Every page defines its glyph outlines as `<path id="font_1_82">` and draws
 * them with `<use xlink:href="#font_1_82">`. Those ids are numbered per page,
 * so two pages in one document collide and the browser resolves every reference
 * to whichever it saw first (glyphs from page one silently appearing on page
 * five). Prefixing each id with its page index keeps them apart.
 *
 * The prefixing happens on the markup, not the DOM: walking every element of a
 * dense page after parsing cost more than the parse itself. The markup is
 * machine generated by mupdf's SVG device and has no text content, so the
 * three patterns below cannot occur inside anything but the attributes they
 * belong to. Only `href="#` is rewritten, not every value starting with "#",
 * because fill="#0000ff" must stay a colour.
 *
 * Parsed via innerHTML rather than DOMParser so the nodes are born in the main
 * document instead of being adopted out of a second one. That is only safe for
 * markup with nothing but tags mupdf emits and no handlers, since mupdf prints
 * some PDF strings (layer and font names) into attributes unescaped. Anything
 * else goes through an inert template and is sanitized first.
 */
function parse(markup: string, pageIndex: number): SVGElement | null {
  const prefix = `p${pageIndex}_`;
  const scoped = markup.replace(/\bid="|href="#|url\(#/g, (match) => match + prefix);

  if (suspicious(scoped)) {
    const template = document.createElement("template");
    template.innerHTML = scoped;
    sanitize(template.content);
    const root = template.content.firstElementChild;
    return root instanceof SVGElement ? document.adoptNode(root) : null;
  }

  const host = document.createElement("div");
  host.innerHTML = scoped;
  const root = host.firstElementChild;
  return root instanceof SVGElement ? root : null;
}

const SVG_NS = "http://www.w3.org/2000/svg";
const SVG_TAGS = new Set([
  "svg", "g", "defs", "clipPath", "mask", "pattern", "path", "rect", "image", "use", "text", "tspan",
]);

function suspicious(markup: string): boolean {
  if (/on[a-z]+\s*=/i.test(markup)) return true;
  for (const [, tag] of markup.matchAll(/<\/?([A-Za-z][\w:-]*)/g)) {
    if (!SVG_TAGS.has(tag)) return true;
  }
  return false;
}

function sanitize(root: DocumentFragment): void {
  for (const element of Array.from(root.querySelectorAll("*"))) {
    if (element.namespaceURI !== SVG_NS || !SVG_TAGS.has(element.localName)) {
      element.remove();
      continue;
    }
    for (const { name } of Array.from(element.attributes)) {
      if (/^on/i.test(name)) element.removeAttribute(name);
    }
  }
}
