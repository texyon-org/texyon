<?php

declare(strict_types=1);

namespace App\Domain\Project;

use DateTimeImmutable;

/** A link that lets whoever opens it join a project. The token itself is never stored. */
final readonly class Invite
{
    public function __construct(
        public int $id,
        public Project $project,
        public Role $role,
        public DateTimeImmutable $createdAt,
    ) {
    }
}
