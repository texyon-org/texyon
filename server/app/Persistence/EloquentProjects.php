<?php

declare(strict_types=1);

namespace App\Persistence;

use App\Domain\Blob\Digest;
use App\Domain\Project\Project;
use App\Domain\Project\ProjectAccess;
use App\Domain\Project\ProjectFile;
use App\Domain\Project\ProjectPath;
use App\Domain\Project\Projects;
use App\Domain\Project\Role;
use App\Persistence\Models\Project as ProjectRow;
use App\Persistence\Models\ProjectFile as ProjectFileRow;
use App\Persistence\Models\ProjectMember as ProjectMemberRow;
use DateTimeImmutable;

/**
 * Projects, in a relational database.
 *
 * The one place models and query builders are allowed. Every method here ends
 * by turning rows into domain objects, so nothing above this file has to know
 * that Eloquent exists — or has to work around what static analysis cannot see
 * through it.
 */
final readonly class EloquentProjects implements Projects
{
    public function ownedBy(int $userId): array
    {
        // array_values, because Collection::all() is only ever typed as
        // array<int, T> and the interface promises a list.
        return array_values(
            ProjectRow::query()
                ->with('owner')
                ->where('owner_id', $userId)
                ->orderByDesc('updated_at')
                ->get()
                ->map(self::toProject(...))
                ->all()
        );
    }

    public function visibleTo(int $userId): array
    {
        $owned = array_map(
            static fn (Project $project): ProjectAccess => new ProjectAccess($project, Role::Owner),
            $this->ownedBy($userId),
        );

        $shared = ProjectMemberRow::query()
            ->with('project.owner')
            ->where('user_id', $userId)
            ->get()
            ->map(static fn (ProjectMemberRow $member): ProjectAccess => new ProjectAccess(
                self::toProject($member->project),
                Role::from($member->role),
            ))
            ->all();

        $all = [...$owned, ...$shared];
        usort($all, static fn (ProjectAccess $a, ProjectAccess $b): int => $b->project->updatedAt <=> $a->project->updatedAt);

        return $all;
    }

    public function findForMember(int $projectId, int $userId): ?ProjectAccess
    {
        $row = ProjectRow::query()->with('owner')->find($projectId);
        if (! $row instanceof ProjectRow) {
            return null;
        }
        if ($row->owner_id === $userId) {
            return new ProjectAccess(self::toProject($row), Role::Owner);
        }

        $role = ProjectMemberRow::query()
            ->where('project_id', $projectId)
            ->where('user_id', $userId)
            ->value('role');

        return is_string($role) ? new ProjectAccess(self::toProject($row), Role::from($role)) : null;
    }

    public function create(int $ownerId, string $name, ProjectPath $primary): Project
    {
        $row = new ProjectRow();
        $row->owner_id = $ownerId;
        $row->name = $name;
        $row->primary_path = $primary->value;
        $row->save();
        $row->load('owner');

        return self::toProject($row);
    }

    public function rename(Project $project, string $name): Project
    {
        $row = ProjectRow::query()->with('owner')->findOrFail($project->id);
        $row->name = $name;
        $row->save();

        return self::toProject($row);
    }

    public function delete(Project $project): void
    {
        ProjectRow::query()->whereKey($project->id)->delete();
    }

    public function files(Project $project): array
    {
        return array_values(
            ProjectFileRow::query()
                ->where('project_id', $project->id)
                ->orderBy('path')
                ->get()
                ->map(self::toFile(...))
                ->all()
        );
    }

    public function file(Project $project, ProjectPath $path): ?ProjectFile
    {
        $row = ProjectFileRow::query()
            ->where('project_id', $project->id)
            ->where('path', $path->value)
            ->first();

        return $row instanceof ProjectFileRow ? self::toFile($row) : null;
    }

    public function putFile(Project $project, ProjectPath $path, Digest $digest, int $size): ProjectFile
    {
        $row = ProjectFileRow::query()
            ->where('project_id', $project->id)
            ->where('path', $path->value)
            ->first() ?? new ProjectFileRow();

        $row->project_id = $project->id;
        $row->path = $path->value;
        $row->digest = $digest->value;
        $row->size = $size;
        $row->save();

        // A file changing is the project changing, and the project list is
        // ordered by when that last happened.
        ProjectRow::query()->whereKey($project->id)->update(['updated_at' => now()]);

        return self::toFile($row);
    }

    public function deleteFile(Project $project, ProjectPath $path): void
    {
        ProjectFileRow::query()
            ->where('project_id', $project->id)
            ->where('path', $path->value)
            ->delete();
    }

    public static function toProject(ProjectRow $row): Project
    {
        return new Project(
            id: $row->id,
            ownerId: $row->owner_id,
            ownerName: $row->owner->name,
            name: $row->name,
            primary: ProjectPath::parse($row->primary_path),
            createdAt: $row->created_at->toDateTimeImmutable(),
            updatedAt: $row->updated_at->toDateTimeImmutable(),
        );
    }

    private static function toFile(ProjectFileRow $row): ProjectFile
    {
        return new ProjectFile(
            id: $row->id,
            projectId: $row->project_id,
            path: ProjectPath::parse($row->path),
            digest: Digest::parse($row->digest),
            size: $row->size,
            updatedAt: $row->updated_at->toDateTimeImmutable(),
        );
    }
}
