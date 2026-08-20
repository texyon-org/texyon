import { useEffect, useState } from "react";

const VIEWABLE: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  bmp: "image/bmp",
  svg: "image/svg+xml",
  pdf: "application/pdf",
};

function bytes(count: number): string {
  if (count < 1024) return `${count} B`;
  if (count < 1024 * 1024) return `${(count / 1024).toFixed(0)} KB`;
  return `${(count / (1024 * 1024)).toFixed(1)} MB`;
}

export function BinaryFileView({ path, content }: { path: string; content: Uint8Array }) {
  const mime = VIEWABLE[path.slice(path.lastIndexOf(".") + 1).toLowerCase()];

  // made and revoked in the effect so a strict mode remount never shows a revoked url
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (mime === undefined) {
      setUrl(null);
      return;
    }
    const made = URL.createObjectURL(new Blob([content as BlobPart], { type: mime }));
    setUrl(made);
    return () => URL.revokeObjectURL(made);
  }, [content, mime]);

  return (
    <div className="binary-view">
      {url !== null && mime === "application/pdf" ? (
        <object data={url} type={mime} title={path} />
      ) : url !== null ? (
        <img src={url} alt={path} />
      ) : null}
      <div className="binary-meta">
        {path} · {bytes(content.length)}
      </div>
    </div>
  );
}
