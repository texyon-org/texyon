# The Tectonic editor, complete: the Laravel API, the single-page client, the
# WebAssembly engine and its support pack, behind one FrankenPHP process.
#
# One image rather than a client image and a server image, because the client
# authenticates with a same-origin session cookie — split them across origins
# and every request becomes third-party. The Caddyfile routes the API to PHP
# and everything else to static files, which is exactly the same-origin layout
# development has behind Vite's proxy.
#
# Build context is the repository root. Two inputs come from outside:
#   client/public/tectonic/  — the engine module and support pack, built by
#                              tools/wasm/build.sh and tools/wasm/build-assets.sh
#                              (a Rust cross-compile; far too slow to redo per image)
#   /srv/bundle (a volume)   — the TeX Live bundle, gigabytes and immutable

# ---------------------------------------------------------------------------
# The client: the js engine wrapper, then the editor built on it.

FROM node:22-bookworm-slim AS client

WORKDIR /src

COPY js/package.json js/package-lock.json js/
RUN cd js && npm ci

COPY js js
RUN cd js && npm run build

COPY client/package.json client/package-lock.json client/
RUN cd client && npm ci

COPY client client

# An editor without an engine is not worth shipping; fail here, with a message,
# rather than serving a page whose Worker 404s.
RUN test -f client/public/tectonic/manifest.json || { \
        echo "client/public/tectonic is missing from the build context." >&2; \
        echo "Run tools/wasm/build.sh and tools/wasm/build-assets.sh first." >&2; \
        exit 1; \
    }

# Which bundle file the engine asks /bundle/ for — the deployment must place
# exactly this file (a .ttb, or a tar with its .index.gz sidecar) in the
# bundle volume. Empty means the client's default.
ARG VITE_BUNDLE_URL=
RUN cd client && VITE_BUNDLE_URL="$VITE_BUNDLE_URL" npm run build

# ---------------------------------------------------------------------------
# The collaboration server, bundled into one file so no node_modules ship.

FROM node:22-bookworm-slim AS collab

WORKDIR /src/collab

COPY collab/package.json collab/package-lock.json ./
RUN npm ci

COPY collab ./
RUN npm run build

# ---------------------------------------------------------------------------
# The server. FrankenPHP rather than nginx and php-fpm, because the static half
# of this is not ordinary: an eighteen-megabyte WebAssembly module, a few
# hundred support files, and range requests against a multi-gigabyte TeX Live
# bundle — which is Caddy's job, done well, in the same process as the API.

FROM dunglas/frankenphp:1-php8.4 AS base

# pdo_sqlite for the default single-tenant deployment; pdo_pgsql for anyone who
# outgrows it. intl and zip are Laravel's usual companions.
RUN install-php-extensions \
        pdo_sqlite \
        pdo_pgsql \
        intl \
        zip \
        opcache

WORKDIR /app

# ---------------------------------------------------------------------------

FROM base AS vendor

COPY --from=composer:2 /usr/bin/composer /usr/bin/composer

# Dependencies are installed against the manifest alone, so editing application
# code does not invalidate this layer.
COPY server/composer.json server/composer.lock ./
RUN composer install \
        --no-dev \
        --no-scripts \
        --no-autoloader \
        --prefer-dist \
        --no-interaction

COPY server .
RUN composer dump-autoload --optimize --no-dev \
    && php artisan package:discover --ansi

# ---------------------------------------------------------------------------

FROM base AS runtime

ENV APP_ENV=production \
    APP_DEBUG=false \
    # The worker keeps the framework booted between requests, which is most of
    # what makes PHP fast and all of what makes it worth using a server that can.
    FRANKENPHP_CONFIG="worker ./public/index.php"

COPY --from=vendor /app /app
# /etc/frankenphp is where the image's entrypoint actually looks; a file left
# in /etc/caddy is silently ignored and the stock config serves instead.
COPY server/Caddyfile /etc/frankenphp/Caddyfile

# The client, laid over Laravel's public directory: index.html and its assets,
# plus the engine and support pack under /tectonic. index.php stays — the
# Caddyfile decides per-path which of the two entry points answers.
COPY --from=client /src/client/dist /app/public

# Node for the collaboration server, beside PHP rather than in its own image.
COPY --from=collab /usr/local/bin/node /usr/local/bin/node
COPY --from=collab /src/collab/dist/server.mjs /app/collab/server.mjs
COPY server/start.sh /app/start.sh
# Fails the build here, not on start, if the base image lacks a library node links against.
RUN node --version

# The database and the blob store are state, and state lives on a volume rather
# than in a layer. The bundle is not even that: the Caddyfile relays it from an
# object store, so nothing here holds it.
VOLUME ["/app/database", "/app/storage", "/srv/blobs"]

RUN mkdir -p /app/storage/framework/{cache,sessions,views} /app/bootstrap/cache \
    && chown -R www-data:www-data /app/storage /app/bootstrap/cache

EXPOSE 80 443

# Migrations run on start rather than as a deploy step someone has to know
# about: they are idempotent, and a fresh volume is the common case here.
CMD ["/app/start.sh"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s \
    CMD curl -fsS http://localhost/up && curl -fsS http://localhost/collab/health || exit 1
