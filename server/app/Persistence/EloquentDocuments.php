<?php

declare(strict_types=1);

namespace App\Persistence;

use App\Domain\Blob\Digest;
use App\Domain\Collab\Documents;
use App\Domain\Collab\StoredDocument;
use App\Domain\Project\Project;
use App\Persistence\Models\ProjectDocument as DocumentRow;
use App\Persistence\Models\ProjectFile as ProjectFileRow;
use Illuminate\Database\UniqueConstraintViolationException;
use Illuminate\Support\Facades\DB;

final readonly class EloquentDocuments implements Documents
{
    public function find(Project $project): ?StoredDocument
    {
        $row = DocumentRow::query()->find($project->id);
        if (! $row instanceof DocumentRow) {
            return null;
        }
        $state = base64_decode($row->state, true);

        return new StoredDocument($state === false ? '' : $state, $row->epoch);
    }

    public function create(Project $project, StoredDocument $document): bool
    {
        try {
            $row = new DocumentRow();
            $row->project_id = $project->id;
            $row->state = base64_encode($document->state);
            $row->epoch = $document->epoch;
            $row->save();
        } catch (UniqueConstraintViolationException) {
            return false;
        }

        return true;
    }

    public function save(Project $project, StoredDocument $document): void
    {
        DocumentRow::query()->updateOrCreate(
            ['project_id' => $project->id],
            ['state' => base64_encode($document->state), 'epoch' => $document->epoch],
        );
    }

    public function recordBlob(Project $project, Digest $digest): void
    {
        DB::table('project_blobs')->insertOrIgnore([
            'project_id' => $project->id,
            'digest' => $digest->value,
            'created_at' => now(),
        ]);
    }

    public function ownsBlob(Project $project, Digest $digest): bool
    {
        return DB::table('project_blobs')->where('project_id', $project->id)->where('digest', $digest->value)->exists()
            || ProjectFileRow::query()->where('project_id', $project->id)->where('digest', $digest->value)->exists();
    }
}
