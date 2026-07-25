<?php

declare(strict_types=1);

namespace App\Domain\Project;

use DateTimeImmutable;

/**
 * One document someone is working on.
 *
 * A readonly view, not an Eloquent model. Everything above the persistence
 * layer is handed one of these, which is what lets static analysis see real
 * types where it would otherwise see a model's magic properties.
 */
final readonly class Project
{
    public function __construct(
        public int $id,
        public int $ownerId,
        public string $name,
        /** The file TeX is pointed at. */
        public ProjectPath $primary,
        public DateTimeImmutable $createdAt,
        public DateTimeImmutable $updatedAt,
    ) {
    }
}
