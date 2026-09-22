<?php

declare(strict_types=1);

namespace App\Persistence;

use App\Domain\Project\Invite;
use App\Domain\Project\Member;
use App\Domain\Project\Project;
use App\Domain\Project\ProjectAccess;
use App\Domain\Project\Role;
use App\Domain\Project\Sharing;
use App\Models\User;
use App\Persistence\Models\ProjectInvite as InviteRow;
use App\Persistence\Models\ProjectMember as MemberRow;

final readonly class EloquentSharing implements Sharing
{
    public function owner(Project $project): Member
    {
        $user = User::query()->findOrFail($project->ownerId);

        return new Member($user->id, $user->name, $user->email, Role::Owner);
    }

    public function members(Project $project): array
    {
        return array_values(
            MemberRow::query()
                ->with('user')
                ->where('project_id', $project->id)
                ->get()
                ->map(self::toMember(...))
                ->sortBy(static fn (Member $member): string => mb_strtolower($member->name))
                ->values()
                ->all()
        );
    }

    public function share(Project $project, string $email, Role $role): ?Member
    {
        // Addresses are stored as typed at sign-up, so compare case-insensitively.
        $user = User::query()->whereRaw('lower(email) = ?', [mb_strtolower($email)])->first();
        if (! $user instanceof User || $user->id === $project->ownerId) {
            return null;
        }

        $row = MemberRow::query()->updateOrCreate(
            ['project_id' => $project->id, 'user_id' => $user->id],
            ['role' => $role->value],
        );
        $row->load('user');

        return self::toMember($row);
    }

    public function setRole(Project $project, int $userId, Role $role): ?Member
    {
        $row = $this->membership($project, $userId);
        if ($row === null) {
            return null;
        }

        $row->role = $role->value;
        $row->save();

        return self::toMember($row);
    }

    public function unshare(Project $project, int $userId): bool
    {
        return MemberRow::query()
            ->where('project_id', $project->id)
            ->where('user_id', $userId)
            ->delete() > 0;
    }

    public function invites(Project $project): array
    {
        return array_values(
            InviteRow::query()
                ->with('project.owner')
                ->where('project_id', $project->id)
                ->orderBy('id')
                ->get()
                ->map(self::toInvite(...))
                ->all()
        );
    }

    public function createInvite(Project $project, Role $role, string $token, int $createdBy): Invite
    {
        $row = new InviteRow();
        $row->project_id = $project->id;
        $row->token_hash = self::hash($token);
        $row->role = $role->value;
        $row->created_by = $createdBy;
        $row->save();
        $row->load('project.owner');

        return self::toInvite($row);
    }

    public function deleteInvite(Project $project, int $inviteId): bool
    {
        return InviteRow::query()
            ->where('project_id', $project->id)
            ->whereKey($inviteId)
            ->delete() > 0;
    }

    public function findInvite(string $token): ?Invite
    {
        $row = InviteRow::query()
            ->with('project.owner')
            ->where('token_hash', self::hash($token))
            ->first();

        return $row instanceof InviteRow ? self::toInvite($row) : null;
    }

    public function accept(Invite $invite, int $userId): ProjectAccess
    {
        $project = $invite->project;
        if ($project->ownerId === $userId) {
            return new ProjectAccess($project, Role::Owner);
        }

        $row = $this->membership($project, $userId);
        if ($row === null) {
            $row = new MemberRow();
            $row->project_id = $project->id;
            $row->user_id = $userId;
            $row->role = $invite->role->value;
            $row->save();
        } elseif (Role::from($row->role) === Role::Viewer && $invite->role === Role::Editor) {
            $row->role = Role::Editor->value;
            $row->save();
        }

        return new ProjectAccess($project, Role::from($row->role));
    }

    private function membership(Project $project, int $userId): ?MemberRow
    {
        $row = MemberRow::query()
            ->with('user')
            ->where('project_id', $project->id)
            ->where('user_id', $userId)
            ->first();

        return $row instanceof MemberRow ? $row : null;
    }

    private static function hash(string $token): string
    {
        return hash('sha256', $token);
    }

    private static function toMember(MemberRow $row): Member
    {
        return new Member($row->user->id, $row->user->name, $row->user->email, Role::from($row->role));
    }

    private static function toInvite(InviteRow $row): Invite
    {
        return new Invite(
            id: $row->id,
            project: EloquentProjects::toProject($row->project),
            role: Role::from($row->role),
            createdAt: $row->created_at->toDateTimeImmutable(),
        );
    }
}
