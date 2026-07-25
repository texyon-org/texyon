<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Blob\BlobStore;
use App\Domain\Project\Project;
use App\Domain\Project\ProjectFile;
use App\Domain\Project\ProjectPath;
use App\Domain\Project\Projects;
use App\Http\Authenticated;
use App\Http\Requests\PutFileRequest;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Support\Facades\Config;
use InvalidArgumentException;
use Symfony\Component\HttpKernel\Exception\BadRequestHttpException;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** The files inside one project. */
final readonly class ProjectFileController
{
    public function __construct(
        private Projects $projects,
        private BlobStore $blobs,
    ) {
    }

    public function index(Request $request, int $project): JsonResponse
    {
        $files = $this->projects->files($this->owned($request, $project));

        return new JsonResponse(['files' => array_map(self::present(...), $files)]);
    }

    /** Read one file's bytes. */
    public function show(Request $request, int $project, string $path): Response|JsonResponse
    {
        $owned = $this->owned($request, $project);
        $file = $this->projects->file($owned, self::path($path));
        if ($file === null) {
            throw new NotFoundHttpException('no such file');
        }

        // Where the store can hand out a signed URL, the bytes never enter this
        // process: a twenty-megabyte figure should not occupy a PHP worker for
        // as long as it takes a browser to download it.
        $direct = $this->blobs->temporaryUrl($file->digest, Config::integer('blobs.url_lifetime'));
        if ($direct !== null) {
            return new JsonResponse(['url' => $direct]);
        }

        return new Response($this->blobs->get($file->digest), 200, [
            'Content-Type' => 'application/octet-stream',
            // The name is the content, so this can never go stale.
            'ETag' => '"'.$file->digest->value.'"',
            'Cache-Control' => 'private, max-age=31536000, immutable',
        ]);
    }

    /** Replace one file's complete contents. */
    public function put(PutFileRequest $request, int $project, string $path): JsonResponse
    {
        $owned = $this->owned($request, $project);
        $contents = $request->contents();

        // Stored before the path is pointed at it, so a listing can never name
        // a digest whose bytes are not there.
        $digest = $this->blobs->put($contents);
        $file = $this->projects->putFile($owned, self::path($path), $digest, strlen($contents));

        return new JsonResponse(['file' => self::present($file)]);
    }

    public function destroy(Request $request, int $project, string $path): JsonResponse
    {
        $this->projects->deleteFile($this->owned($request, $project), self::path($path));

        // The blob is deliberately left behind: another file, in this project
        // or another, may be the same bytes. Reclaiming them is a sweep over
        // what nothing references, not a side effect of one delete.
        return new JsonResponse(status: 204);
    }

    private function owned(Request $request, int $projectId): Project
    {
        $project = $this->projects->findForOwner($projectId, Authenticated::id($request));
        if ($project === null) {
            throw new NotFoundHttpException('no such project');
        }

        return $project;
    }

    private static function path(string $raw): ProjectPath
    {
        try {
            return ProjectPath::parse($raw);
        } catch (InvalidArgumentException $error) {
            throw new BadRequestHttpException($error->getMessage(), $error);
        }
    }

    /** @return array<string, string|int> */
    private static function present(ProjectFile $file): array
    {
        return [
            'path' => $file->path->value,
            'digest' => $file->digest->value,
            'size' => $file->size,
            'updatedAt' => $file->updatedAt->format(DATE_ATOM),
        ];
    }
}
