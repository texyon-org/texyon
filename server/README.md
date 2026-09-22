# The Tectonic editor server

Accounts, projects, and somewhere to put files. Nothing else: LaTeX is compiled
in the browser by `@tectonic/wasm`, so this never runs TeX, never holds a
document open, and scales like an ordinary CRUD application because that is all
it is.

## What it does

- **Auth** — Laravel Fortify, headless. It accepts the POSTs behind login,
  registration, password reset, two-factor and passkeys; the client renders the
  screens.
- **Projects and files** — a project owns a set of paths, and each path names
  some content.
- **Content storage** — content-addressed, on any disk Flysystem can reach.
- **Static hosting** — the client, the WebAssembly engine, its support pack, and
  range requests against a TeX Live bundle.

## The shape of the code

Two rules do most of the work.

**Eloquent stops at `app/Persistence`.** Everything above it is handed readonly
DTOs — `Project`, `ProjectFile`, `Digest`, `ProjectPath` — whose types static
analysis can actually see. Models, query builders and magic properties live in
one directory instead of running through the application. This is what makes
PHPStan level 10 affordable without annotating around the ORM.

**Blobs are content-addressed, and `BlobStore` is not a filesystem.** S3 has no
rename, no partial write and no directories; an interface shaped like POSIX
would either lie about that or leak it. What a project needs is narrower: store
bytes, get them back by what they are, and hand a browser a way to fetch them
without the bytes passing through PHP. Two files with identical contents are one
blob, and a digest is safe to cache forever because nothing can change under it
without becoming a different digest.

## Checks

```
composer analyse   # PHPStan level 10, no baseline
php artisan test
composer check     # both
```

Level 10 is PHPStan's maximum: an implicit `mixed` is an error. There is no
baseline, deliberately — a baseline is a list of things nobody intends to fix,
and starting one on the first commit makes the level a decoration.

Exactly one strict rule is turned off, in `phpstan.neon` with the reason beside
it: `dynamicCallOnStaticMethod`, because Eloquent's builder forwards to methods
the framework declares static, so `Project::query()->orderBy(...)` is an
instance call to a static method *by design*. It is disabled once rather than
suppressed in every repository method.

## Serving the client

`Caddyfile` is not boilerplate. Three things in it are load-bearing, and each
corresponds to something the browser engine will do:

- **Range requests must work.** The client reads individual files out of the
  bundle tar with one range request each, and refuses a `200` where it asked for
  a `206` — otherwise a server that ignored the header would hand it several
  gigabytes. Compression is off for that path: encoding the response makes the
  byte offsets the client computed meaningless.
- **A missing support file must be a 404.** The single-page-app fallback is
  scoped so it can never reach `/tectonic/*`. Without that, the engine is handed
  an HTML document and told it is `pdftex.map`.
- **The engine needs a validator.** The client revalidates its cached copy of
  the eighteen-megabyte module against `ETag` or `Last-Modified`; with neither,
  it re-downloads every load rather than risk running a stale engine. Caddy sets
  both.

No `Cross-Origin-Opener-Policy` or `Cross-Origin-Embedder-Policy` is needed. The
engine blocks with synchronous `XMLHttpRequest` rather than `SharedArrayBuffer`,
which keeps cross-origin isolation — and everything it breaks — out of the
picture.

## Running it

The image is the whole editor — this API, the built client, the WebAssembly
engine and its support pack — and CI publishes it to
`ghcr.io/yendric/tectonic-editor` on every push to `master` (or on demand via
the *Editor Image* workflow). Deploying is pulling it:

```
cp .env.example .env          # set APP_KEY: php artisan key:generate --show
docker compose up
```

To build the image yourself instead, the engine assets have to exist first —
they are a Rust cross-compile that lives outside the Dockerfile:

```
tools/wasm/build.sh && tools/wasm/build-assets.sh   # once, from the repo root
docker compose up --build
```

The bundle is deliberately not in the image: it is measured in gigabytes and is
the same for everyone. It is not even in a volume, for now — the Caddyfile
relays `/bundle/*` to the object store holding `texlive2026.ttb`, so a deploy
needs nothing beyond the image itself. To self-host the bundle instead, swap
the commented block in the Caddyfile, restore the `/srv/bundle` mount in
`compose.yaml`, and put the file there.

SQLite is the default because for a single tenant it is the right answer — a
file, no second process, and WAL handles the concurrency of a few people editing
their own documents. The queries are portable; `DB_CONNECTION=pgsql` is the
whole change for an installation that outgrows it.

## Sharing and live editing

An owner shares a project with an existing account by email, or with an invite
link anyone signed in can open, as an editor or a viewer. Members, roles and
links live in `project_members` and `project_invites`, and every project
endpoint resolves access through `ProjectGate`: no access is a 404, too little
access a 403.

Editing is live. Each project has one Yjs document, served by the Node server in
`collab/` (Hocuspocus) and stored here in `project_documents`:

- The editor asks `POST /api/projects/{id}/collab-token` for a signed pass,
  valid for a minute and fetched again on every reconnect, then opens a
  websocket to `/collab`. Viewers get a read-only connection, and the server
  drops their edits.
- The collaboration server loads and stores documents through
  `/api/internal/collab/*`, which Caddy only answers from inside the container.
  Both sides derive the shared secret from `APP_KEY`, so there is nothing extra
  to configure.
- A first load seeds the document from `project_files`. Every store writes the
  files back, so `project_files` stays a plain copy of the document and nothing
  else has to understand Yjs. Once a project has a document, the whole-file
  write endpoints answer 409.
- Images and other binaries are uploaded to `PUT /api/projects/{id}/blobs` and
  named in the document by digest. A document can only name blobs its own
  project uploaded.
- Changing someone's role, removing them, deleting the project, deleting an
  account and signing out all disconnect the affected editors. They reconnect
  with a fresh token, or learn they no longer have access.

In the image, `start.sh` runs the collaboration server beside FrankenPHP and
stops the container if either exits. In development, run it next to
`php artisan serve` and Vite:

```
cd collab && npm ci
LARAVEL_URL=http://127.0.0.1:8080 LARAVEL_ENV_FILE=../server/.env npm run dev
```
