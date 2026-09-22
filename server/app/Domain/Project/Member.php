<?php

declare(strict_types=1);

namespace App\Domain\Project;

/** Someone with access to a project, the owner included. */
final readonly class Member
{
    public function __construct(
        public int $userId,
        public string $name,
        public string $email,
        public Role $role,
    ) {
    }
}
