"use strict";

/**
 * What the engine holds of each source, and how to bring it up to date.
 *
 * A file the engine holds at its base revision gets one splice. Anything
 * else gets its whole text, so a missed or reordered change can never leave
 * the engine with the wrong bytes.
 */
class SourceTracker {
  constructor() {
    /** @type {Map<string, {revision: number, held: number | null, sent: number | null, edit: object | null, read: () => string}>} */
    this.files = new Map();
  }

  /** Track a source, read through `read` whenever its whole text is needed. */
  track(path, read) {
    const file = this.files.get(path);
    if (file) {
      file.read = read;
      return;
    }
    this.files.set(path, { revision: 1, held: null, sent: null, edit: null, read });
  }

  /** Record one change event as VS Code reports it, in UTF-16 offsets. */
  change(path, changes) {
    const file = this.files.get(path);
    if (!file) return;
    // A compile in flight will leave the engine at `sent`, so a splice can build on it.
    const base = file.sent ?? file.held;
    const spliceable = changes.length === 1 && base === file.revision && file.edit === null;
    file.edit = spliceable
      ? {
        base: file.revision,
        from: changes[0].rangeOffset,
        to: changes[0].rangeOffset + changes[0].rangeLength,
        insert: changes[0].text
      }
      : null;
    file.revision += 1;
  }

  /** Whether the engine is behind on any source. */
  get stale() {
    for (const file of this.files.values()) if (file.held !== file.revision) return true;
    return false;
  }

  /** The `files`, `edits` and `texts` of a compile request, which is now in flight. */
  manifest() {
    const files = {};
    const edits = {};
    const texts = {};
    for (const [path, file] of this.files) {
      files[path] = file.revision;
      file.sent = file.revision;
      if (file.held === file.revision) continue;
      if (file.edit && file.held === file.edit.base) edits[path] = file.edit;
      else texts[path] = file.read();
      file.edit = null;
    }
    return { files, edits, texts };
  }

  /** The engine accepted the manifest in flight. */
  acknowledge(manifest) {
    for (const [path, revision] of Object.entries(manifest.files)) {
      const file = this.files.get(path);
      if (!file) continue;
      file.held = revision;
      file.sent = null;
    }
  }

  /** Forget what the engine holds, so the next compile sends every text. */
  reset() {
    for (const file of this.files.values()) {
      file.held = null;
      file.sent = null;
      file.edit = null;
    }
  }
}

module.exports = { SourceTracker };
