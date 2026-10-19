"use strict";

function parseServerLine(line) {
  try {
    return { event: JSON.parse(line) };
  } catch (error) {
    if (line.startsWith('note: "version 2" TeXyon command-line interface activated')) {
      return {
        error:
          "configured TeXyon executable does not support live preview; " +
          "rebuild it with `cargo build --release -p texyon --features preview-mupdf`"
      };
    }
    return { error: `server emitted invalid JSON: ${error.message}` };
  }
}

module.exports = { parseServerLine };
