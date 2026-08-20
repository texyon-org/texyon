import { BoltIcon } from "./icons";

/** The FlashTeX wordmark, with the TeX ligature's lowered E. */
export function Wordmark({ size = 18 }: { size?: number }) {
  return (
    <span className="wordmark" style={{ fontSize: `${size}px` }}>
      <BoltIcon size={size + 2} />
      <span className="wordmark-text">
        Flash
        <span className="wordmark-tex">
          T<span className="wordmark-e">E</span>X
        </span>
      </span>
    </span>
  );
}
