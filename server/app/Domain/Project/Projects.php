<?php

declare(strict_types=1);

namespace App\Domain\Project;

use App\Domain\Blob\Digest;

/**
 * Everything the application does to projects.
 *
 * The boundary Eloquent stops at. Implementations may do whatever they like
 * with models and query builders; what they hand back is readonly and typed.
 */
interface Projects
{
    /**
     * Every project someone owns, newest first.
     *
     * @return list<Project>
     */
    public function ownedBy(int $userId): array;

    /**
     * Every project someone owns or has been given, newest first.
     *
     * @return list<ProjectAccess>
     */
    public function visibleTo(int $userId): array;

    /**
     * One project and the role that user has in it, if any.
     *
     * Access is part of the lookup rather than a check afterwards, so there is
     * no way to fetch a project and forget to ask who is asking.
     */
    public function findForMember(int $projectId, int $userId): ?ProjectAccess;

    public function create(int $ownerId, string $name, ProjectPath $primary): Project;

    public function rename(Project $project, string $name): Project;

    public function delete(Project $project): void;

    /**
     * Every file in a project, by path.
     *
     * @return list<ProjectFile>
     */
    public function files(Project $project): array;

    public function file(Project $project, ProjectPath $path): ?ProjectFile;

    /**
     * Point a path at some stored bytes, creating the entry or replacing it.
     *
     * The blob must already be in the store: a path may only ever name content
     * that exists, so a listing can never promise bytes nobody can fetch.
     */
    public function putFile(Project $project, ProjectPath $path, Digest $digest, int $size): ProjectFile;

    public function deleteFile(Project $project, ProjectPath $path): void;
}
