# Deferred live-preview experiments

The production design intentionally stops at sound command-fence replay,
zero-lookahead viewport horizons, and selective page rendering. The following
ideas may be explored in isolated benchmarks, but have no production hooks:

- paragraph-local layout overlays that do not run normal page building;
- output reconnection based on page, auxiliary-file, or state convergence;
- parallel cloned engines or background global compilation;
- non-whole-buffer editing protocols;
- browser-native or Windows-native SVG backends behind the portable renderer
  contract.

An experiment must first define a cold-batch differential oracle, an explicit
failure fallback, state ownership, memory accounting, cancellation points, and
a measured workload where the existing fence/horizon design is insufficient.
It must not add dormant flags, alternate viewer beliefs, engine display lists,
or speculative tail hints to production code. Promotion requires simplifying
the chosen architecture or replacing it outright, plus the full gate matrix.
