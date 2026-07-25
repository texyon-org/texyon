<?php

declare(strict_types=1);

namespace App\Domain\Project;

use App\Domain\Blob\Digest;
use DateTimeImmutable;

/**
 * One file in a project, and where its bytes are.
 *
 * The bytes themselves are not here. A project listing names every file and a
 * client asks for the few it wants to open, so carrying content through the
 * listing would make opening a project proportional to its size rather than to
 * its file count.
 */
final readonly class ProjectFile
{
    public function __construct(
        public int $id,
        public int $projectId,
        public ProjectPath $path,
        public Digest $digest,
        public int $size,
        public DateTimeImmutable $updatedAt,
    ) {
    }
}
