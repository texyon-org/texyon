<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Blob\BlobStore;
use App\Domain\Project\ProjectAccess;
use App\Domain\Project\Projects;
use App\Domain\Project\Role;
use App\Http\Authenticated;
use App\Http\Presenters\ProjectPresenter;
use App\Http\ProjectGate;
use App\Http\Requests\StoreProjectRequest;
use Dedoc\Scramble\Attributes\Response as ResponseShape;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;

/** The projects a signed-in person owns or has been given. */
final readonly class ProjectController
{
    /** What a project starts as, so there is something to compile from the first moment. */
    private const SKELETON = "\\documentclass{article}\n\n\\begin{document}\n\nHello.\n\n\\end{document}\n";

    public function __construct(
        private Projects $projects,
        private ProjectGate $gate,
        private BlobStore $blobs,
    ) {}

    // Scramble cannot see through array_map.
    #[ResponseShape(200, type: 'array{projects: list<'.ProjectPresenter::PROJECT.'>}')]
    public function index(Request $request): JsonResponse
    {
        $projects = $this->projects->visibleTo(Authenticated::id($request));

        return new JsonResponse([
            'projects' => array_map(ProjectPresenter::project(...), $projects),
        ]);
    }

    public function store(StoreProjectRequest $request): JsonResponse
    {
        $project = $this->projects->create(
            Authenticated::id($request),
            $request->name(),
            $request->primary(),
        );
        $this->projects->putFile($project, $project->primary, $this->blobs->put(self::SKELETON), strlen(self::SKELETON));

        return new JsonResponse(['project' => ProjectPresenter::project(new ProjectAccess($project, Role::Owner))], 201);
    }

    public function show(Request $request, int $project): JsonResponse
    {
        return new JsonResponse(['project' => ProjectPresenter::project($this->gate->read($request, $project))]);
    }

    public function destroy(Request $request, int $project): Response
    {
        $this->projects->delete($this->gate->manage($request, $project)->project);

        return response()->noContent();
    }
}
