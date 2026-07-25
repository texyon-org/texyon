#!/usr/bin/env bash
# Fetch TeXstudio's completion word lists.
#
# The corpus describes what LaTeX commands *mean* — what their arguments are
# called, where they are legal, which package provides them — which is the one
# thing the engine's own macro table cannot tell us. See
# docs/editor/completion.md.
#
# It is fetched rather than vendored, and neither it nor anything built from it
# is committed. TeXstudio is GPL-3.0 and this tree is MIT; until that is
# resolved the dependency stays at arm's length, where removing it is a matter
# of deleting a directory.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLONE="$HERE/texstudio"

if [ -d "$CLONE/.git" ]; then
    echo "== updating"
    git -C "$CLONE" fetch --depth 1 origin master
    git -C "$CLONE" checkout -q FETCH_HEAD
else
    echo "== cloning (completion/ only)"
    # Blobless and sparse: the completion directory is 24 MB, the whole
    # repository is a great deal more and none of the rest is wanted.
    git clone --depth 1 --filter=blob:none --sparse \
        https://github.com/texstudio-org/texstudio.git "$CLONE"
    git -C "$CLONE" sparse-checkout set completion
fi

echo "== $(find "$CLONE/completion" -name '*.cwl' | wc -l) cwl files"
echo
echo "Now run: node --experimental-strip-types tools/cwl/build.mjs"
