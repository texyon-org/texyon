import type { CompileResult, Link } from "@tectonic/wasm";
import { useCallback, useEffect, useRef, useState } from "react";

/** Zoom steps, in the order the buttons walk through them. */
const ZOOM_STEPS = [0.5, 0.75, 1, 1.25, 1.5, 2, 3] as const;

export interface ViewerProps {
  /** The latest compile, or null before the first one. */
  result: CompileResult | null;
  /** Called once the changed pages are on screen. */
  onPainted: () => void;
  /** Someone clicked a point on a page; find the source line that made it. */
  onBackwardSearch: (pageIndex: number, hBp: number, vBp: number) => void;
  /** Where a forward search says the cursor's line landed. */
  scrollTo: { pageIndex: number; vBp: number } | null;
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
export function Viewer({ result, onPainted, onBackwardSearch, scrollTo, onViewport }: ViewerProps) {
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
    const host = container.current;
    if (result === null || host === null) return;

    while (slots.current.length < result.pageCount) {
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
    while (slots.current.length > result.pageCount) {
      slots.current.pop()?.remove();
    }

    // Size every slot, including ones we were not sent, so the scroll height is
    // right before their pages arrive.
    result.pageSizes.forEach(([width, height], index) => {
      const slot = slots.current[index];
      if (slot === undefined) return;
      slot.style.width = `${width * zoom}px`;
      slot.style.height = `${height * zoom}px`;
    });

    for (const page of result.pages) {
      const slot = slots.current[page.index];
      if (slot === undefined) continue;

      const svg = parse(page.svg, page.index);
      if (svg === null) continue;

      svg.setAttribute("width", "100%");
      svg.setAttribute("height", "100%");
      slot.replaceChildren(svg);
    }

    // Links are not drawn — they arrive beside the page as regions — so they go
    // on as a layer of anchors above the SVG. Every page's overlay is rebuilt,
    // because a page whose text did not move can still have had its link
    // renumbered by an edit above it, and because the zoom may have changed.
    result.pageOverlays.forEach((overlay, index) => {
      const slot = slots.current[index];
      if (slot === undefined) return;

      slot.querySelector(".links")?.remove();
      if (overlay.links.length === 0) return;

      const layer = document.createElement("div");
      layer.className = "links";
      for (const link of overlay.links) {
        layer.appendChild(anchorFor(link, zoom, result, slots.current, scroller.current));
      }
      slot.appendChild(layer);
    });

    setPageCount(result.pageCount);

    // The document may have grown, shrunk or been rescaled under a still
    // scrollbar, all of which change which pages are visible.
    measureViewport();

    if (result.pages.length > 0) {
      // Two frames: one to let the browser lay the new page out, one to be sure
      // it has painted it.
      requestAnimationFrame(() => requestAnimationFrame(onPainted));
    }
  }, [result, zoom, onPainted, pointOf, measureViewport]);

  // A forward search: the cursor's line landed here.
  useEffect(() => {
    if (scrollTo === null) return;
    const slot = slots.current[scrollTo.pageIndex];
    const view = scroller.current;
    if (slot === undefined || view == null) return;

    view.scrollTo({ top: slot.offsetTop + scrollTo.vBp * zoom - view.clientHeight / 3, behavior: "smooth" });
    slot.classList.add("located");
    const timer = window.setTimeout(() => slot.classList.remove("located"), 900);
    return () => window.clearTimeout(timer);
  }, [scrollTo, zoom]);

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
 * to whichever it saw first — glyphs from page one silently appearing on page
 * five. Prefixing each id with its page index keeps them apart.
 */
function parse(markup: string, pageIndex: number): SVGElement | null {
  const parsed = new DOMParser().parseFromString(markup, "image/svg+xml");
  if (parsed.querySelector("parsererror") !== null) return null;

  const root = parsed.documentElement;
  if (!(root instanceof SVGElement)) return null;

  const prefix = `p${pageIndex}_`;
  for (const element of root.querySelectorAll("[id]")) {
    const id = element.getAttribute("id");
    if (id !== null) element.setAttribute("id", prefix + id);
  }

  // Only href and url(#…) are rewritten. Rewriting anything that merely starts
  // with "#" also rewrites fill="#0000ff" into a colour that does not parse,
  // and the browser silently falls back to black.
  for (const element of root.querySelectorAll("*")) {
    for (const attribute of [...element.attributes]) {
      const value = attribute.value;
      if ((attribute.name === "href" || attribute.name.endsWith(":href")) && value.startsWith("#")) {
        attribute.value = `#${prefix}${value.slice(1)}`;
      } else if (value.includes("url(#")) {
        attribute.value = value.replace(/url\(#/g, `url(#${prefix}`);
      }
    }
  }

  return root;
}
