import { useMemo, useState } from "react";

import { CaretIcon, CrossIcon, PencilIcon, PlusIcon } from "../ui/icons";

export interface FileTreeProps {
  paths: readonly string[];
  /** The file currently open. */
  open: string;
  /** The project's primary file, which is marked and cannot be removed. */
  primary: string;
  /** Paths with unsaved changes. */
  dirty: ReadonlySet<string>;
  onOpen: (path: string) => void;
  onCreate: (path: string) => void;
  onRemove: (path: string) => void;
  onRename: (from: string, to: string) => void;
}

/** A directory, or a file, in the tree that paths imply. */
interface Node {
  name: string;
  path: string;
  children: Node[];
  isFile: boolean;
}

/**
 * The project's files.
 *
 * A project is a flat map of paths; the tree is inferred from the slashes in
 * them rather than stored, because a directory with nothing in it is not
 * something a project can have.
 */
export function FileTree({
  paths,
  open,
  primary,
  dirty,
  onOpen,
  onCreate,
  onRemove,
  onRename,
}: FileTreeProps) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");

  const tree = useMemo(() => build(paths), [paths]);

  const submit = (): void => {
    const path = draft.trim();
    setAdding(false);
    setDraft("");
    if (path !== "") onCreate(path);
  };

  return (
    <nav className="files">
      <header className="files-bar">
        <span>Files</span>
        <button onClick={() => setAdding(true)} title="New file">
          <PlusIcon />
        </button>
      </header>

      {adding && (
        <input
          className="files-new"
          autoFocus
          placeholder="chapters/one.tex"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={submit}
          onKeyDown={(event) => {
            if (event.key === "Enter") submit();
            if (event.key === "Escape") {
              setAdding(false);
              setDraft("");
            }
          }}
        />
      )}

      <ul className="files-list">
        {tree.map((node) => (
          <TreeNode
            key={node.path}
            node={node}
            depth={0}
            open={open}
            primary={primary}
            dirty={dirty}
            onOpen={onOpen}
            onRemove={onRemove}
            onRename={onRename}
          />
        ))}
      </ul>
    </nav>
  );
}

function TreeNode({
  node,
  depth,
  open,
  primary,
  dirty,
  onOpen,
  onRemove,
  onRename,
}: {
  node: Node;
  depth: number;
  open: string;
  primary: string;
  dirty: ReadonlySet<string>;
  onOpen: (path: string) => void;
  onRemove: (path: string) => void;
  onRename: (from: string, to: string) => void;
}) {
  const [expanded, setExpanded] = useState(true);

  if (!node.isFile) {
    return (
      <li className="files-dir">
        <button
          className="files-row"
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
          onClick={() => setExpanded((was) => !was)}
        >
          <CaretIcon open={expanded} />
          {node.name}
        </button>
        {expanded && (
          <ul>
            {node.children.map((child) => (
              <TreeNode
                key={child.path}
                node={child}
                depth={depth + 1}
                open={open}
                primary={primary}
                dirty={dirty}
                onOpen={onOpen}
                onRemove={onRemove}
                onRename={onRename}
              />
            ))}
          </ul>
        )}
      </li>
    );
  }

  const isPrimary = node.path === primary;

  return (
    <li>
      <div className={`files-row file ${node.path === open ? "current" : ""}`}>
        <button
          className="files-name"
          style={{ paddingLeft: `${depth * 12 + 8}px` }}
          onClick={() => onOpen(node.path)}
          title={node.path}
        >
          {node.name}
          {dirty.has(node.path) && <span className="files-dirty" title="Unsaved">•</span>}
          {isPrimary && <span className="files-primary" title="The file TeX is pointed at">main</span>}
        </button>

        <button
          className="files-action"
          title="Rename"
          onClick={() => {
            const to = window.prompt("Rename to", node.path);
            if (to !== null && to.trim() !== "") onRename(node.path, to.trim());
          }}
        >
          <PencilIcon />
        </button>

        {/* The primary is what TeX is pointed at; a project without one cannot
            compile, so it is not offered for deletion. */}
        {!isPrimary && (
          <button
            className="files-action"
            title="Delete"
            onClick={() => {
              if (window.confirm(`Delete ${node.path}?`)) onRemove(node.path);
            }}
          >
            <CrossIcon size={12} />
          </button>
        )}
      </div>
    </li>
  );
}

/** Infer the directory tree from the slashes in a flat list of paths. */
function build(paths: readonly string[]): Node[] {
  const roots: Node[] = [];

  for (const path of [...paths].sort()) {
    const segments = path.split("/");
    let level = roots;
    let prefix = "";

    segments.forEach((name, index) => {
      prefix = prefix === "" ? name : `${prefix}/${name}`;
      const isFile = index === segments.length - 1;

      let node = level.find((candidate) => candidate.name === name && candidate.isFile === isFile);
      if (node === undefined) {
        node = { name, path: prefix, children: [], isFile };
        level.push(node);
      }
      level = node.children;
    });
  }

  // Directories first, then files, each alphabetically — the order a person
  // reading a tree expects rather than the order the paths happened to sort in.
  const order = (nodes: Node[]): Node[] => {
    nodes.sort((left, right) =>
      left.isFile === right.isFile
        ? left.name.localeCompare(right.name)
        : Number(left.isFile) - Number(right.isFile),
    );
    for (const node of nodes) order(node.children);
    return nodes;
  };

  return order(roots);
}
