<?php

declare(strict_types=1);

namespace App\Collab;

use Illuminate\Http\Client\ConnectionException;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\Http;
use Illuminate\Support\Facades\Log;

/**
 * Tells the collaboration server to drop connections that lost their access.
 *
 * Best effort: when it cannot be reached there is nothing to drop, and a
 * reconnect asks Laravel for a token again anyway.
 */
final readonly class CollabServer
{
    public function kickProject(int $projectId): void
    {
        $this->kick(['project' => $projectId]);
    }

    public function kickUser(int $userId, ?int $projectId = null): void
    {
        $this->kick($projectId === null ? ['user' => $userId] : ['user' => $userId, 'project' => $projectId]);
    }

    /** @param array<string, int> $target */
    private function kick(array $target): void
    {
        try {
            Http::timeout(2)
                ->withHeaders(['X-Collab-Secret' => CollabSecret::hex()])
                ->post(rtrim(Config::string('collab.url'), '/').'/collab/internal/kick', $target);
        } catch (ConnectionException $error) {
            Log::info('collab server unreachable for a kick', ['error' => $error->getMessage()]);
        }
    }
}
