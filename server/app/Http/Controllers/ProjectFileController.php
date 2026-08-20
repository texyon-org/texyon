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
use Dedoc\Scramble\Attributes\ExcludeRouteFromDocs;
use Dedoc\Scramble\Attributes\QueryParameter;
use Dedoc\Scramble\Attributes\Response as ResponseShape;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\URL;
use InvalidArgumentException;
use Symfony\Component\HttpKernel\Exception\BadRequestHttpException;
use Symfony\Component\HttpKernel\Exception\HttpException;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** The files inside one project. `path` is a query parameter because file names contain slashes. */
final readonly class ProjectFileController
{
    public function __construct(
        private Projects $projects,
        private BlobStore $blobs,
    ) {}

    // Must match present(). Scramble cannot see through array_map.
    #[ResponseShape(200, type: 'array{files: list<array{path: string, digest: string, size: int, updatedAt: string}>}')]
    public function index(Request $request, int $project): JsonResponse
    {
        $files = $this->projects->files($this->owned($request, $project));

        return new JsonResponse(['files' => array_map(self::present(...), $files)]);
    }

    /** Where to fetch one file's bytes. Always a URL, signed by S3 or by the server itself. */
    #[QueryParameter('path', description: 'The file inside the project, e.g. `chapters/one.tex`.', required: true, type: 'string')]
    public function show(Request $request, int $project): JsonResponse
    {
        $owned = $this->owned($request, $project);
        $file = $this->projects->file($owned, self::path($request));
        if ($file === null) {
            throw new NotFoundHttpException('no such file');
        }

        $lifetime = Config::integer('blobs.url_lifetime');

        // Relative, so it stays on the client's origin.
        $url = $this->blobs->temporaryUrl($file->digest, $lifetime)
            ?? URL::temporarySignedRoute(
                'blobs.read',
                now()->addSeconds($lifetime),
                ['digest' => $file->digest->value],
                absolute: false,
            );

        return new JsonResponse(['url' => $url]);
    }

    /** Replace one file's complete contents. */
    #[QueryParameter('path', description: 'The file inside the project, e.g. `chapters/one.tex`.', required: true, type: 'string')]
    public function put(PutFileRequest $request, int $project): JsonResponse
    {
        $owned = $this->owned($request, $project);
        $contents = $request->contents();

        // Store the bytes before pointing the path at them.
        $digest = $this->blobs->put($contents);
        $file = $this->projects->putFile($owned, self::path($request), $digest, strlen($contents));

        return new JsonResponse(['file' => self::present($file)]);
    }

    /** Replace one file with the raw request body. Not in the OpenAPI export, the client binding is hand written. */
    #[ExcludeRouteFromDocs]
    public function putBytes(Request $request, int $project): JsonResponse
    {
        $owned = $this->owned($request, $project);
        $contents = $request->getContent();

        // raw bodies bypass FormRequest validation, enforce the cap here
        if (strlen($contents) > Config::integer('blobs.max_file_bytes')) {
            throw new HttpException(413, 'the file is larger than the server accepts');
        }

        $digest = $this->blobs->put($contents);
        $file = $this->projects->putFile($owned, self::path($request), $digest, strlen($contents));

        return new JsonResponse(['file' => self::present($file)]);
    }

    #[QueryParameter('path', description: 'The file inside the project, e.g. `chapters/one.tex`.', required: true, type: 'string')]
    public function destroy(Request $request, int $project): Response
    {
        $this->projects->deleteFile($this->owned($request, $project), self::path($request));

        // The blob stays, other files may share the same bytes.
        return response()->noContent();
    }

    private function owned(Request $request, int $projectId): Project
    {
        $project = $this->projects->findForOwner($projectId, Authenticated::id($request));
        if ($project === null) {
            throw new NotFoundHttpException('no such project');
        }

        return $project;
    }

    private static function path(Request $request): ProjectPath
    {
        $raw = $request->query('path');
        if (! is_string($raw)) {
            throw new BadRequestHttpException('a path query parameter is required');
        }

        try {
            return ProjectPath::parse($raw);
        } catch (InvalidArgumentException $error) {
            throw new BadRequestHttpException($error->getMessage(), $error);
        }
    }

    /** @return array{path: string, digest: string, size: int, updatedAt: string} */
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
