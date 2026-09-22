<?php

declare(strict_types=1);

namespace App\Collab;

use Illuminate\Support\Facades\Config;

/** The key shared with the collaboration server, derived from APP_KEY exactly as collab/src/secret.ts does. */
final class CollabSecret
{
    public static function bytes(): string
    {
        $key = Config::string('app.key');
        if (str_starts_with($key, 'base64:')) {
            $key = base64_decode(substr($key, 7), true);
            if ($key === false) {
                throw new \RuntimeException('APP_KEY is not valid base64');
            }
        }

        return hash_hmac('sha256', 'tectonic-collab', $key, true);
    }

    public static function hex(): string
    {
        return bin2hex(self::bytes());
    }

    public static function matches(?string $presented): bool
    {
        return $presented !== null && hash_equals(self::hex(), $presented);
    }
}
