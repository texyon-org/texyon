# The Tectonic editor server: an API, a place to put files, and a static host.
#
# FrankenPHP rather than nginx and php-fpm, because the static half of this is
# not ordinary. It serves an eighteen-megabyte WebAssembly module, a few hundred
# support files, and range requests against a multi-gigabyte TeX Live bundle —
# which is Caddy's job, done well, in the same process as the API.

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
COPY composer.json composer.lock ./
RUN composer install \
        --no-dev \
        --no-scripts \
        --no-autoloader \
        --prefer-dist \
        --no-interaction

COPY . .
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
COPY Caddyfile /etc/frankenphp/Caddyfile

# The database and the blob store are state, and state lives on a volume rather
# than in a layer. The bundle is separate again: it is measured in gigabytes and
# has no business in an image.
VOLUME ["/app/database", "/app/storage", "/srv/blobs", "/srv/bundle"]

RUN mkdir -p /app/storage/framework/{cache,sessions,views} /app/bootstrap/cache \
    && chown -R www-data:www-data /app/storage /app/bootstrap/cache

EXPOSE 80 443

HEALTHCHECK --interval=30s --timeout=5s --start-period=20s \
    CMD curl -fsS http://localhost/up || exit 1
