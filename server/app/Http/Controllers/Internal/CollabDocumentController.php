<?php

declare(strict_types=1);

namespace App\Http\Controllers\Internal;

use App\Collab\Materializer;
use App\Domain\Collab\Documents;
use App\Domain\Project\Project;
use App\Domain\Project\Projects;
use App\Http\Requests\StoreDocumentRequest;
use Dedoc\Scramble\Attributes\ExcludeRouteFromDocs;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** What the collaboration server loads documents from and stores them to. Not part of the public API. */
final readonly class CollabDocumentController
{
    public function __construct(
        private Projects $projects,
        private Documents $documents,
        private Materializer $materializer,
    ) {}

    /** The stored document, or when there is none yet, the files to seed one from. */
    #[ExcludeRouteFromDocs]
    public function show(int $project): JsonResponse
    {
        $target = $this->project($project);
        $stored = $this->documents->find($target);

        return new JsonResponse([
            'state' => $stored === null ? null : base64_encode($stored->state),
            'epoch' => $stored?->epoch,
            ...($stored === null ? $this->materializer->contents($target) : ['files' => [], 'binaries' => []]),
        ]);
    }

    /** Store a document and bring project_files in line with it. `?create=1` only stores a first one. */
    #[ExcludeRouteFromDocs]
    public function put(StoreDocumentRequest $request, int $project): JsonResponse
    {
        $target = $this->project($project);
        $document = $request->document();

        if ($request->boolean('create')) {
            if (! $this->documents->create($target, $document)) {
                return new JsonResponse(['message' => 'this project has a document already'], 409);
            }
        } else {
            $this->documents->save($target, $document);
        }
        $this->materializer->apply($target, $request->files(), $request->binaries());

        return new JsonResponse(['stored' => true]);
    }

    private function project(int $projectId): Project
    {
        $project = $this->projects->find($projectId);
        if ($project === null) {
            throw new NotFoundHttpException('no such project');
        }

        return $project;
    }
}
