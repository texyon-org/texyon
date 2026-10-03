<?php

declare(strict_types=1);

namespace App\Domain\Project;

use App\Domain\Blob\BlobStore;
use App\Domain\Blob\Digest;
use InvalidArgumentException;
use ZipArchive;

/** One project's files read out of a zip, like the ones Overleaf exports, already in the blob store. */
final readonly class ProjectArchive
{
    /**
     * @param  list<array{path: ProjectPath, digest: Digest, size: int}>  $files
     * @param  list<array{path: string, reason: string}>  $skipped
     */
    private function __construct(
        public array $files,
        public array $skipped,
        public ProjectPath $primary,
    ) {}

    /** @throws InvalidArgumentException if the zip cannot be read or holds nothing usable */
    public static function read(string $zipPath, BlobStore $blobs, int $maxFileBytes, int $maxTotalBytes, int $maxEntries): self
    {
        $zip = new ZipArchive;
        if ($zip->open($zipPath, ZipArchive::RDONLY) !== true) {
            throw new InvalidArgumentException('the upload is not a zip archive');
        }

        try {
            if ($zip->numFiles > $maxEntries) {
                throw new InvalidArgumentException("the archive holds more than {$maxEntries} entries");
            }

            /** @var list<array{name: string, index: int, size: int}> $entries */
            $entries = [];
            for ($index = 0; $index < $zip->numFiles; $index++) {
                $stat = $zip->statIndex($index);
                if ($stat === false || str_ends_with($stat['name'], '/') || self::isJunk($stat['name'])) {
                    continue;
                }
                $entries[] = ['name' => $stat['name'], 'index' => $index, 'size' => $stat['size']];
            }

            $root = self::commonRoot(array_column($entries, 'name'));

            $files = [];
            $skipped = [];
            $documents = [];
            $total = 0;
            foreach ($entries as $entry) {
                $name = substr($entry['name'], strlen($root));
                try {
                    $path = ProjectPath::parse($name);
                } catch (InvalidArgumentException) {
                    $skipped[] = ['path' => $name, 'reason' => 'invalid path'];

                    continue;
                }
                if ($entry['size'] > $maxFileBytes) {
                    $skipped[] = ['path' => $path->value, 'reason' => 'too large'];

                    continue;
                }

                $total += $entry['size'];
                if ($total > $maxTotalBytes) {
                    throw new InvalidArgumentException('the archive unpacks to more than the server accepts');
                }

                // capped at the declared size, a lying header cannot inflate past it
                $contents = $zip->getFromIndex($entry['index'], $entry['size']);
                if ($contents === false) {
                    $skipped[] = ['path' => $path->value, 'reason' => 'unreadable'];

                    continue;
                }

                unset($documents[$path->value]);
                if ($path->isTex() && preg_match('/^[^%\n]*\\\\documentclass/m', $contents) === 1) {
                    $documents[$path->value] = $path;
                }
                // later entries win, as in a client-side import
                $files[$path->value] = ['path' => $path, 'digest' => $blobs->put($contents), 'size' => strlen($contents)];
            }
        } finally {
            $zip->close();
        }

        if ($files === []) {
            throw new InvalidArgumentException('the archive holds no files that could be imported');
        }

        return new self(array_values($files), $skipped, self::primaryOf(array_keys($files), array_values($documents)));
    }

    /** Filesystem litter that travels inside zips. */
    private static function isJunk(string $name): bool
    {
        foreach (explode('/', str_replace('\\', '/', $name)) as $segment) {
            if ($segment === '__MACOSX' || $segment === '.DS_Store' || $segment === 'Thumbs.db'
                || str_starts_with($segment, '._') || str_ends_with($segment, ':Zone.Identifier')) {
                return true;
            }
        }

        return false;
    }

    /**
     * A top-level directory every entry lives under, with its slash, or ''.
     *
     * @param  list<string>  $names
     */
    private static function commonRoot(array $names): string
    {
        $root = null;
        foreach ($names as $name) {
            $at = strpos($name, '/');
            if ($at === false || $at === 0) {
                return '';
            }
            $first = substr($name, 0, $at + 1);
            if ($root === null) {
                $root = $first;
            } elseif ($root !== $first) {
                return '';
            }
        }

        return $root ?? '';
    }

    /**
     * main.tex when there is one, else the shallowest file with a \documentclass.
     *
     * @param  list<string>  $paths
     * @param  list<ProjectPath>  $documents
     */
    private static function primaryOf(array $paths, array $documents): ProjectPath
    {
        $fallback = ProjectPath::parse('main.tex');
        if (in_array($fallback->value, $paths, true)) {
            return $fallback;
        }

        usort($documents, static fn (ProjectPath $a, ProjectPath $b): int => [substr_count($a->value, '/'), $a->value] <=> [substr_count($b->value, '/'), $b->value]);

        return $documents[0] ?? $fallback;
    }
}
