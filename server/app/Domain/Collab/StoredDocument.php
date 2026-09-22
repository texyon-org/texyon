<?php

declare(strict_types=1);

namespace App\Domain\Collab;

/** A project's shared document as last stored: a Yjs update, and the epoch it was seeded in. */
final readonly class StoredDocument
{
    public function __construct(
        public string $state,
        public string $epoch,
    ) {}
}
