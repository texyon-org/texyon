# Security

If you find a security problem in TeXyoN, please email texyon@yendric.be
instead of opening a public issue. I'll reply within a few days and keep you
posted until it's fixed.

Things worth reporting include a document that can write files outside its
project, run commands, crash the engine in a way that corrupts memory, or
escape the web editor's worker. Like any TeX engine, the native build lets a
document read files by absolute path, for example to load installed fonts.

Only the latest release gets security fixes.
