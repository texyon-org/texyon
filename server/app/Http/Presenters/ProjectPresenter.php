<?php

declare(strict_types=1);

namespace App\Http\Presenters;

use App\Domain\Project\Invite;
use App\Domain\Project\Member;
use App\Domain\Project\ProjectAccess;

/** The JSON shapes shared by the project, member and invite endpoints. */
final class ProjectPresenter
{
    public const PROJECT = 'array{id: int, name: string, primary: string, role: string, owner: array{name: string}, createdAt: string, updatedAt: string}';

    public const MEMBER = 'array{id: int, name: string, email: string, role: string}';

    public const INVITE = 'array{id: int, role: string, createdAt: string}';

    /** @return array{id: int, name: string, primary: string, role: string, owner: array{name: string}, createdAt: string, updatedAt: string} */
    public static function project(ProjectAccess $access): array
    {
        $project = $access->project;

        return [
            'id' => $project->id,
            'name' => $project->name,
            'primary' => $project->primary->value,
            'role' => $access->role->value,
            'owner' => ['name' => $project->ownerName],
            'createdAt' => $project->createdAt->format(DATE_ATOM),
            'updatedAt' => $project->updatedAt->format(DATE_ATOM),
        ];
    }

    /** @return array{id: int, name: string, email: string, role: string} */
    public static function member(Member $member): array
    {
        return [
            'id' => $member->userId,
            'name' => $member->name,
            'email' => $member->email,
            'role' => $member->role->value,
        ];
    }

    /** @return array{id: int, role: string, createdAt: string} */
    public static function invite(Invite $invite): array
    {
        return [
            'id' => $invite->id,
            'role' => $invite->role->value,
            'createdAt' => $invite->createdAt->format(DATE_ATOM),
        ];
    }
}
