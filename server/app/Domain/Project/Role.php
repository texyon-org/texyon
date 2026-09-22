<?php

declare(strict_types=1);

namespace App\Domain\Project;

/** What someone may do with a project. */
enum Role: string
{
    case Owner = 'owner';
    case Editor = 'editor';
    case Viewer = 'viewer';

    public function canWrite(): bool
    {
        return $this !== self::Viewer;
    }

    public function canManage(): bool
    {
        return $this === self::Owner;
    }
}
