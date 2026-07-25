<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Project\Project;
use App\Domain\Project\Projects;
use App\Http\Authenticated;
use App\Http\Requests\StoreProjectRequest;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** The projects a signed-in person owns. */
final readonly class ProjectController
{
    public function __construct(private Projects $projects)
    {
    }

    public function index(Request $request): JsonResponse
    {
        $projects = $this->projects->ownedBy(Authenticated::id($request));

        return new JsonResponse([
            'projects' => array_map(self::present(...), $projects),
        ]);
    }

    public function store(StoreProjectRequest $request): JsonResponse
    {
        $project = $this->projects->create(
            Authenticated::id($request),
            $request->name(),
            $request->primary(),
        );

        return new JsonResponse(['project' => self::present($project)], 201);
    }

    public function show(Request $request, int $project): JsonResponse
    {
        return new JsonResponse(['project' => self::present($this->owned($request, $project))]);
    }

    public function destroy(Request $request, int $project): JsonResponse
    {
        $this->projects->delete($this->owned($request, $project));

        return new JsonResponse(status: 204);
    }

    /**
     * Fetch a project the requester owns, or stop.
     *
     * Not found rather than forbidden: whether a project exists is itself
     * something only its owner should be able to learn.
     */
    private function owned(Request $request, int $projectId): Project
    {
        $project = $this->projects->findForOwner($projectId, Authenticated::id($request));
        if ($project === null) {
            throw new NotFoundHttpException('no such project');
        }

        return $project;
    }

    /** @return array<string, string|int> */
    private static function present(Project $project): array
    {
        return [
            'id' => $project->id,
            'name' => $project->name,
            'primary' => $project->primary->value,
            'createdAt' => $project->createdAt->format(DATE_ATOM),
            'updatedAt' => $project->updatedAt->format(DATE_ATOM),
        ];
    }
}
