(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.TectonicViewerState = api;
})(typeof globalThis === "object" ? globalThis : this, function () {
  "use strict";

  const MAX_U64 = (1n << 64n) - 1n;
  const BLOCKED_ELEMENTS = new Set([
    "script", "style", "foreignobject", "iframe", "object", "embed", "audio", "video",
    "a", "animate", "animatemotion", "animatetransform", "discard", "mpath", "set"
  ]);

  function sequence(value, name) {
    if (typeof value !== "string" || !/^(0|[1-9][0-9]*)$/.test(value)) {
      throw new Error(`${name} is not a canonical decimal string`);
    }
    const parsed = BigInt(value);
    if (parsed > MAX_U64) throw new Error(`${name} exceeds u64`);
    return parsed;
  }

  function initialState() {
    return {
      lastFrameId: null,
      sourceRevision: null,
      pages: [],
      visible: null,
      prefetch: [],
      pageSizes: [],
      tailPending: false,
      phase: null,
      outcome: null
    };
  }

  function validateIndices(indices, bound, label) {
    if (!Array.isArray(indices)) throw new Error(`${label} is not an array`);
    let previous = -1;
    for (const index of indices) {
      if (!Number.isSafeInteger(index) || index < 0 || index >= bound || index <= previous) {
        throw new Error(`${label} indices are not strictly increasing within bounds`);
      }
      previous = index;
    }
  }

  function validateSizes(sizes, count) {
    if (!Array.isArray(sizes) || sizes.length !== count) {
      throw new Error("page_count disagrees with page_sizes");
    }
    for (const size of sizes) {
      if (!size || !Number.isFinite(size.width_bp) || !Number.isFinite(size.height_bp) ||
          size.width_bp <= 0 || size.height_bp <= 0) {
        throw new Error("page size is not finite and positive");
      }
    }
  }

  /** Validate and atomically reduce one server update. */
  function applyUpdate(current, event, sanitize) {
    if (!event || event.type !== "update") throw new Error("viewer reducer requires an update");
    const frame = sequence(event.frame_id, "frame_id");
    const revision = sequence(event.source_revision, "source_revision");
    if (current.lastFrameId !== null && frame <= sequence(current.lastFrameId, "last frame_id")) {
      throw new Error("frame_id did not strictly increase");
    }
    if (current.sourceRevision !== null && revision < sequence(current.sourceRevision, "previous source_revision")) {
      throw new Error("source_revision decreased");
    }
    const count = event.page_count;
    if (!Number.isSafeInteger(count) || count < 0) throw new Error("invalid page_count");
    validateSizes(event.page_sizes, count);
    validateIndices(event.changed, count, "changed");
    validateIndices(event.removed, current.pages.length, "removed");
    const expectedRemoved = [];
    for (let index = count; index < current.pages.length; index += 1) expectedRemoved.push(index);
    if (JSON.stringify(event.removed) !== JSON.stringify(expectedRemoved)) {
      throw new Error("removed pages are not exactly the truncated suffix");
    }
    if (!Array.isArray(event.pages_svg)) throw new Error("pages_svg is not an array");
    const payloads = new Map();
    for (const payload of event.pages_svg) {
      const index = payload && payload.page_index;
      if (!Number.isSafeInteger(index) || index < 0 || index >= count ||
          typeof payload.svg !== "string" || payloads.has(index)) {
        throw new Error("invalid or duplicate SVG payload");
      }
      payloads.set(index, payload.svg);
    }
    if (JSON.stringify([...payloads.keys()].sort((a, b) => a - b)) !== JSON.stringify(event.changed)) {
      throw new Error("changed pages disagree with SVG payloads");
    }

    const pages = current.pages.slice(0, count).map((page) => ({ ...page }));
    while (pages.length < count) pages.push({ displayedSvg: null, pendingSvg: null });
    for (const index of event.changed) {
      const safe = sanitize(payloads.get(index), `f${event.frame_id}_p${index}`);
      if (current.prefetch.includes(index)) {
        pages[index].displayedSvg = safe;
        pages[index].pendingSvg = null;
      } else {
        pages[index].pendingSvg = safe;
      }
    }
    return {
      state: {
        lastFrameId: event.frame_id,
        sourceRevision: event.source_revision,
        pages,
        visible: current.visible,
        prefetch: current.prefetch.filter((index) => index < count),
        pageSizes: event.page_sizes.map((size) => ({ ...size })),
        tailPending: !!event.tail_pending,
        phase: event.phase,
        outcome: event.outcome
      },
      acknowledgement: event.frame_id
    };
  }

  /** Record the true visible range without conflating it with prefetch. */
  function setVisible(current, first, last) {
    if (!Number.isSafeInteger(first) || !Number.isSafeInteger(last) || first < 0 || first > last) {
      throw new Error("invalid visible range");
    }
    return { ...current, visible: { first, last } };
  }

  /** Install pending payloads in the observer's one-viewport prefetch band. */
  function setPrefetch(current, indices) {
    validateIndices(indices, current.pages.length, "prefetch");
    const pages = current.pages.map((page) => ({ ...page }));
    const installed = [];
    for (const index of indices) {
      if (pages[index].pendingSvg !== null) {
        pages[index].displayedSvg = pages[index].pendingSvg;
        pages[index].pendingSvg = null;
        installed.push(index);
      }
    }
    return { state: { ...current, pages, prefetch: indices.slice() }, installed };
  }

  function preflightSvg(svg) {
    if (typeof svg !== "string" || svg.length === 0) throw new Error("empty SVG");
    if (/<!doctype|<\?|<!entity/i.test(svg)) throw new Error("doctype, entity, or processing instruction is forbidden");
    if (/<\s*\/?\s*(script|style|foreignobject|iframe|object|embed|audio|video|a|animate|animatemotion|animatetransform|discard|mpath|set)\b/i.test(svg)) {
      throw new Error("active SVG element is forbidden");
    }
    if (/\son[a-z0-9_-]+\s*=/i.test(svg)) throw new Error("event attribute is forbidden");
    if (/javascript\s*:|data\s*:\s*text\/html|@import|expression\s*\(/i.test(svg)) {
      throw new Error("active SVG attribute is forbidden");
    }
    for (const match of svg.matchAll(/\b(?:xlink:)?href\s*=\s*(['"])(.*?)\1/gi)) {
      rewriteHref(match[2], "preflight");
    }
    rewriteUrlFunctions(svg, "preflight");
    return svg;
  }

  function rewriteUrlFunctions(value, prefix) {
    return value.replace(/url\(\s*(['"]?)(.*?)\1\s*\)/gi, (_all, _quote, target) => {
      if (!target.startsWith("#") || target.length === 1) throw new Error("external url() is forbidden");
      return `url(#${prefix}__${target.slice(1)})`;
    });
  }

  function rewriteHref(value, prefix) {
    if (value.startsWith("#") && value.length > 1) return `#${prefix}__${value.slice(1)}`;
    if (/^data:image\/(png|jpeg|jpg);base64,[a-z0-9+/=\s]+$/i.test(value)) return value;
    throw new Error("external href is forbidden");
  }

  /** Browser sanitizer: reject active content, namespace IDs, and import one SVG. */
  function sanitizeSvg(svg, prefix, DOMParserType, XMLSerializerType) {
    preflightSvg(svg);
    if (!/^[A-Za-z0-9_-]+$/.test(prefix)) throw new Error("invalid SVG namespace");
    const documentNode = new DOMParserType().parseFromString(svg, "image/svg+xml");
    if (documentNode.doctype || documentNode.getElementsByTagName("parsererror").length > 0) {
      throw new Error("malformed SVG");
    }
    const root = documentNode.documentElement;
    if (!root || root.localName.toLowerCase() !== "svg") throw new Error("root element is not SVG");
    const visit = (element) => {
      if (BLOCKED_ELEMENTS.has(element.localName.toLowerCase())) throw new Error("active SVG element is forbidden");
      for (const attribute of Array.from(element.attributes)) {
        const name = attribute.name.toLowerCase();
        let value = attribute.value;
        if (name.startsWith("on")) throw new Error("event attribute is forbidden");
        if (name === "id") {
          if (value.length === 0) throw new Error("empty SVG id");
          value = `${prefix}__${value}`;
        } else if (name === "href" || name.endsWith(":href")) {
          value = rewriteHref(value, prefix);
        } else {
          value = rewriteUrlFunctions(value, prefix);
        }
        element.setAttributeNS(attribute.namespaceURI, attribute.name, value);
      }
      for (const child of Array.from(element.children)) visit(child);
    };
    visit(root);
    return new XMLSerializerType().serializeToString(root);
  }

  return {
    applyUpdate,
    initialState,
    preflightSvg,
    rewriteHref,
    rewriteUrlFunctions,
    sanitizeSvg,
    sequence,
    setPrefetch,
    setVisible
  };
});
