#!/bin/bash
# The editor image's entry point: migrations, then the collaboration server and
# FrankenPHP side by side. If either stops, the container stops, so Docker's
# restart policy brings both back instead of leaving half an editor running.
set -euo pipefail

php artisan migrate --force --path=migrations

# The browser origin the collaboration server accepts, unless set explicitly.
export COLLAB_ALLOWED_ORIGINS="${COLLAB_ALLOWED_ORIGINS:-${APP_URL:-}}"
export LARAVEL_URL="${LARAVEL_URL:-http://127.0.0.1}"

node /app/collab/server.mjs &
collab=$!
frankenphp run --config /etc/frankenphp/Caddyfile --adapter caddyfile &
php=$!

trap 'kill -TERM "$collab" "$php" 2>/dev/null' TERM INT
wait -n "$collab" "$php"
status=$?
kill -TERM "$collab" "$php" 2>/dev/null || true
wait
exit "$status"
