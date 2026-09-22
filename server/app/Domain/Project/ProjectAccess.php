<?php

declare(strict_types=1);

namespace App\Domain\Project;

/** A project as one particular person may use it. */
final readonly class ProjectAccess
{
    public function __construct(
        public Project $project,
        public Role $role,
    ) {
    }
}
