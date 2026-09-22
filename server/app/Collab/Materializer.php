<?php

declare(strict_types=1);

namespace App\Collab;

use App\Domain\Blob\BlobMissing;
use App\Domain\Blob\BlobStore;
use App\Domain\Blob\Digest;
use App\Domain\Collab\Documents;
use App\Domain\Project\Project;
use App\Domain\Project\ProjectPath;
use App\Domain\Project\Projects;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;

/**
 * Moves files between project_files and a shared document.
 *
 * The document is what people edit. project_files is kept as its plain copy, so
 * anything that reads files never needs to understand Yjs.
 */
final readonly class Materializer
{
    public function __construct(
        private Projects $projects,
        private BlobStore $blobs,
        private Documents $documents,
    ) {}

    /**
     * What a fresh document is seeded from. Text is anything that is valid UTF-8 without NUL bytes,
     * the same rule the client applies when it imports files.
     *
     * @return array{files: list<array{path: string, text: string}>, binaries: list<array{path: string, digest: string, size: int}>}
     */
    public function contents(Project $project): array
    {
        $files = [];
        $binaries = [];
        foreach ($this->projects->files($project) as $file) {
            try {
                $bytes = $this->blobs->get($file->digest);
            } catch (BlobMissing) {
                continue;
            }
            if (mb_check_encoding($bytes, 'UTF-8') && ! str_contains($bytes, "\0")) {
                $files[] = ['path' => $file->path->value, 'text' => $bytes];
            } else {
                $binaries[] = ['path' => $file->path->value, 'digest' => $file->digest->value, 'size' => $file->size];
            }
        }

        return ['files' => $files, 'binaries' => $binaries];
    }

    /**
     * Make project_files match a document. Paths that do not parse, and blobs the
     * project never uploaded, are skipped rather than failing the whole store.
     *
     * @param  list<array{path: string, text: string}>  $files
     * @param  list<array{path: string, digest: string, size: int}>  $binaries
     */
    public function apply(Project $project, array $files, array $binaries): void
    {
        /** @var array<string, array{ProjectPath, Digest, int}> $wanted */
        $wanted = [];
        foreach ($files as $file) {
            $path = self::path($file['path']);
            if ($path !== null) {
                $wanted[$path->value] = [$path, $this->blobs->put($file['text']), strlen($file['text'])];
            }
        }
        foreach ($binaries as $binary) {
            $path = self::path($binary['path']);
            $digest = self::digest($binary['digest']);
            if ($path !== null && $digest !== null && $this->documents->ownsBlob($project, $digest)) {
                $wanted[$path->value] = [$path, $digest, $binary['size']];
            }
        }

        DB::transaction(function () use ($project, $wanted): void {
            $existing = [];
            foreach ($this->projects->files($project) as $file) {
                $existing[$file->path->value] = $file->digest->value;
                if (! isset($wanted[$file->path->value])) {
                    $this->projects->deleteFile($project, $file->path);
                }
            }
            foreach ($wanted as $key => [$path, $digest, $size]) {
                if (($existing[$key] ?? null) !== $digest->value) {
                    $this->projects->putFile($project, $path, $digest, $size);
                }
            }
        });
    }

    private static function path(string $raw): ?ProjectPath
    {
        try {
            return ProjectPath::parse($raw);
        } catch (InvalidArgumentException) {
            return null;
        }
    }

    private static function digest(string $raw): ?Digest
    {
        try {
            return Digest::parse($raw);
        } catch (InvalidArgumentException) {
            return null;
        }
    }
}
