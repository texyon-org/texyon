<?php

declare(strict_types=1);

namespace App\Domain\Collab;

use App\Domain\Blob\Digest;
use App\Domain\Project\Project;

/** Where shared documents, and the blobs each project may point at, are kept. */
interface Documents
{
    public function find(Project $project): ?StoredDocument;

    /** Store a project's first document. False when one exists already. */
    public function create(Project $project, StoredDocument $document): bool;

    public function save(Project $project, StoredDocument $document): void;

    public function recordBlob(Project $project, Digest $digest): void;

    /** Whether the project uploaded these bytes, or already has a file made of them. */
    public function ownsBlob(Project $project, Digest $digest): bool;
}
