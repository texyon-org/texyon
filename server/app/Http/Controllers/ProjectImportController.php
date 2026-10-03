<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Blob\BlobStore;
use App\Domain\Project\ProjectAccess;
use App\Domain\Project\ProjectArchive;
use App\Domain\Project\Projects;
use App\Domain\Project\Role;
use App\Http\Authenticated;
use App\Http\Presenters\ProjectPresenter;
use Dedoc\Scramble\Attributes\ExcludeRouteFromDocs;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\DB;
use InvalidArgumentException;
use RuntimeException;
use Symfony\Component\HttpKernel\Exception\HttpException;
use Symfony\Component\HttpKernel\Exception\UnprocessableEntityHttpException;

/** A new project from one zip in the raw request body. Not in the OpenAPI export, the client binding is hand written. */
final readonly class ProjectImportController
{
    public function __construct(
        private Projects $projects,
        private BlobStore $blobs,
    ) {}

    #[ExcludeRouteFromDocs]
    public function store(Request $request): JsonResponse
    {
        $name = trim($request->string('name')->value());
        if ($name === '' || mb_strlen($name) > 120) {
            throw new UnprocessableEntityHttpException('a name of at most 120 characters is required');
        }

        $maxArchive = Config::integer('blobs.max_import_bytes');
        $zipPath = $this->spool($request, $maxArchive);

        try {
            $archive = ProjectArchive::read($zipPath, $this->blobs, Config::integer('blobs.max_file_bytes'), $maxArchive, 20_000);
        } catch (InvalidArgumentException $error) {
            throw new UnprocessableEntityHttpException($error->getMessage(), $error);
        } finally {
            @unlink($zipPath);
        }

        $project = DB::transaction(function () use ($request, $name, $archive) {
            $project = $this->projects->create(Authenticated::id($request), $name, $archive->primary);
            foreach ($archive->files as $file) {
                $this->projects->putFile($project, $file['path'], $file['digest'], $file['size']);
            }

            return $project;
        });

        return new JsonResponse([
            'project' => ProjectPresenter::project(new ProjectAccess($project, Role::Owner)),
            'skipped' => $archive->skipped,
        ], 201);
    }

    /** The body to a temporary file, so a large zip never sits in memory whole. */
    private function spool(Request $request, int $maxBytes): string
    {
        $declared = $request->header('Content-Length');
        if (is_numeric($declared) && (int) $declared > $maxBytes) {
            throw new HttpException(413, 'the archive is larger than the server accepts');
        }

        $path = tempnam(sys_get_temp_dir(), 'import');
        $target = $path === false ? false : fopen($path, 'wb');
        if ($path === false || $target === false) {
            throw new RuntimeException('cannot create a temporary file for the import');
        }

        $source = $request->getContent(true);
        $copied = stream_copy_to_stream($source, $target, $maxBytes + 1);
        fclose($target);

        if ($copied === false || $copied > $maxBytes) {
            @unlink($path);
            throw new HttpException(413, 'the archive is larger than the server accepts');
        }

        return $path;
    }
}
