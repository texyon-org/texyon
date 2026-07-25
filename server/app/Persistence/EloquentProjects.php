<?php

declare(strict_types=1);

namespace App\Persistence;

use App\Domain\Blob\Digest;
use App\Domain\Project\Project;
use App\Domain\Project\ProjectFile;
use App\Domain\Project\ProjectPath;
use App\Domain\Project\Projects;
use App\Persistence\Models\Project as ProjectRow;
use App\Persistence\Models\ProjectFile as ProjectFileRow;
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
                ->where('owner_id', $userId)
                ->orderByDesc('updated_at')
                ->get()
                ->map(self::toProject(...))
                ->all()
        );
    }

    public function findForOwner(int $projectId, int $userId): ?Project
    {
        $row = ProjectRow::query()
            ->where('id', $projectId)
            ->where('owner_id', $userId)
            ->first();

        return $row instanceof ProjectRow ? self::toProject($row) : null;
    }

    public function create(int $ownerId, string $name, ProjectPath $primary): Project
    {
        $row = new ProjectRow();
        $row->owner_id = $ownerId;
        $row->name = $name;
        $row->primary_path = $primary->value;
        $row->save();

        return self::toProject($row);
    }

    public function rename(Project $project, string $name): Project
    {
        $row = ProjectRow::query()->findOrFail($project->id);
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

    private static function toProject(ProjectRow $row): Project
    {
        return new Project(
            id: $row->id,
            ownerId: $row->owner_id,
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
