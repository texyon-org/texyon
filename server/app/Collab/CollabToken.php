<?php

declare(strict_types=1);

namespace App\Collab;

use App\Domain\Project\ProjectAccess;
use Illuminate\Support\Facades\Config;

/** A short-lived pass to one project's document, read by collab/src/token.ts. */
final class CollabToken
{
    /** @return array{token: string, expiresAt: int} */
    public static function issue(ProjectAccess $access, int $userId, string $name): array
    {
        $expiresAt = time() + Config::integer('collab.token_ttl');
        $json = json_encode([
            'uid' => $userId,
            'pid' => $access->project->id,
            'role' => $access->role->value,
            'name' => $name,
            'exp' => $expiresAt,
        ], JSON_THROW_ON_ERROR);

        $body = self::base64url($json);
        $signature = self::base64url(hash_hmac('sha256', $body, CollabSecret::bytes(), true));

        return ['token' => "{$body}.{$signature}", 'expiresAt' => $expiresAt];
    }

    private static function base64url(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }
}
