import { useEffect, useRef, useState } from "react";
import type { CompileResult } from "@tectonic/wasm";

/** Display scale. One SVG user unit is one PostScript big point. */
const SCALE = 1.35;

export interface PreviewProps {
  /** The latest compile, or null before the first one. */
  result: CompileResult | null;
  /** Called once the changed pages are on screen. */
  onPainted: () => void;
}

/**
 * The rendered document.
 *
 * The engine hands back a *patch* — only the pages whose appearance changed —
 * so this keeps a DOM slot per page and replaces the contents of just those.
 * A one-word edit in a twenty-page document touches one slot; the other
 * nineteen are never re-parsed, never re-laid-out and never repainted.
 */
export function Preview({ result, onPainted }: PreviewProps) {
  const container = useRef<HTMLDivElement>(null);
  const slots = useRef<HTMLElement[]>([]);
  const [pageCount, setPageCount] = useState(0);

  useEffect(() => {
    const host = container.current;
    if (!result || !host) return;

    // Grow or shrink the slot list to match the document. Slots that survive
    // keep their contents, which is the entire point.
    while (slots.current.length < result.pageCount) {
      const slot = document.createElement("section");
      slot.className = "page";
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
      if (!slot) return;
      slot.style.width = `${width * SCALE}px`;
      slot.style.height = `${height * SCALE}px`;
    });

    for (const page of result.pages) {
      const slot = slots.current[page.index];
      if (!slot) continue;

      const svg = parse(page.svg, page.index);
      if (!svg) continue;

      svg.setAttribute("width", "100%");
      svg.setAttribute("height", "100%");
      slot.replaceChildren(svg);
    }

    // Links are not drawn — they arrive beside the page as regions — so they go
    // on as a layer of anchors above the SVG. Every page's overlay is rebuilt,
    // not just the changed ones: a page whose text did not move can still have
    // had its link renumbered by an edit above it.
    result.pageOverlays.forEach((overlay, index) => {
      const slot = slots.current[index];
      if (!slot) return;

      slot.querySelector(".links")?.remove();
      if (overlay.links.length === 0) return;

      const layer = document.createElement("div");
      layer.className = "links";

      for (const link of overlay.links) {
        const anchor = document.createElement("a");
        anchor.style.left = `${link.left * SCALE}px`;
        anchor.style.top = `${link.top * SCALE}px`;
        anchor.style.width = `${(link.right - link.left) * SCALE}px`;
        anchor.style.height = `${(link.bottom - link.top) * SCALE}px`;

        if (link.uri) {
          anchor.href = link.uri;
          anchor.target = "_blank";
          // The document is untrusted input; an opened tab must not reach back.
          anchor.rel = "noreferrer noopener";
          anchor.title = link.uri;
        } else if (link.destination) {
          anchor.href = `#${link.destination}`;
          anchor.title = link.destination;
          anchor.onclick = (event) => {
            event.preventDefault();
            jumpTo(link.destination!, result, slots.current);
          };
        }

        layer.appendChild(anchor);
      }

      slot.appendChild(layer);
    });

    setPageCount(result.pageCount);

    if (result.pages.length > 0) {
      // Two frames: one to let the browser lay the new page out, one to be sure
      // it has painted it.
      requestAnimationFrame(() => requestAnimationFrame(onPainted));
    }
  }, [result, onPainted]);

  const empty = pageCount === 0;

  return (
    <div className="preview">
      {empty && <div className="preview-empty">Waiting for the first compile…</div>}
      <div className="preview-pages" ref={container} />
      {!empty && (
        <div className="preview-footer">
          {pageCount} page{pageCount === 1 ? "" : "s"}
        </div>
      )}
    </div>
  );
}

/**
 * Scroll to a named destination, wherever in the document it landed.
 *
 * Destinations are per page, so following one means finding which page declared
 * the name before anything can be scrolled to.
 */
function jumpTo(name: string, result: CompileResult, slots: HTMLElement[]): void {
  for (const [index, overlay] of result.pageOverlays.entries()) {
    const target = overlay.destinations.find((d) => d.name === name);
    if (!target) continue;

    const slot = slots[index];
    if (!slot) return;

    // The destination is a point on the page, so scroll the page into view and
    // then offset by where on it the target sits.
    const top = slot.offsetTop + target.yBp * SCALE;
    slot.parentElement?.parentElement?.scrollTo({ top: top - 40, behavior: "smooth" });
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
  if (parsed.querySelector("parsererror")) return null;

  const svg = parsed.documentElement;
  if (!(svg instanceof SVGElement)) return null;

  const prefix = `p${pageIndex}_`;

  for (const element of svg.querySelectorAll("[id]")) {
    element.id = prefix + element.id;
  }

  // References take two forms: an href of "#id" on <use>, and url(#id) inside
  // presentation attributes such as clip-path and mask.
  //
  // Only those two. Rewriting every attribute that merely *starts* with "#"
  // also rewrites `fill="#0000ff"` into a colour that does not parse, and the
  // browser silently falls back to black — which looks exactly like a renderer
  // that lost its colours.
  for (const element of svg.querySelectorAll("*")) {
    for (const attribute of Array.from(element.attributes)) {
      const isReference =
        attribute.localName === "href" &&
        (attribute.namespaceURI === null ||
          attribute.namespaceURI === "http://www.w3.org/1999/xlink");

      if (isReference && attribute.value.startsWith("#")) {
        attribute.value = `#${prefix}${attribute.value.slice(1)}`;
      } else if (attribute.value.includes("url(#")) {
        attribute.value = attribute.value.replaceAll("url(#", `url(#${prefix}`);
      }
    }
  }

  return svg;
}
