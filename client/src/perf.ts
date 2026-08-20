/** performance.measure that tolerates a start mark that was never made. */
export function measure(name: string, start: string, end: string): void {
  try {
    performance.measure(name, start, end);
  } catch {
    // nothing to measure yet
  }
}
