<?php

declare(strict_types=1);

namespace App\Domain\Project;

/** Who besides the owner may use a project, and the links that let more people in. */
interface Sharing
{
    public function owner(Project $project): Member;

    /**
     * Everyone the project is shared with, by name. The owner is not included.
     *
     * @return list<Member>
     */
    public function members(Project $project): array;

    /**
     * Share with an existing account, or change the role of someone already in.
     *
     * Null when no account has that address. The owner cannot be added.
     */
    public function share(Project $project, string $email, Role $role): ?Member;

    /** Null when that user is not a member. */
    public function setRole(Project $project, int $userId, Role $role): ?Member;

    /** Whether there was a membership to remove. */
    public function unshare(Project $project, int $userId): bool;

    /** @return list<Invite> */
    public function invites(Project $project): array;

    public function createInvite(Project $project, Role $role, string $token, int $createdBy): Invite;

    /** Whether there was an invite to delete. */
    public function deleteInvite(Project $project, int $inviteId): bool;

    public function findInvite(string $token): ?Invite;

    /**
     * Join through an invite. Never lowers a role someone already has.
     */
    public function accept(Invite $invite, int $userId): ProjectAccess;
}
