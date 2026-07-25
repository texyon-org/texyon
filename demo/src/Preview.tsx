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
