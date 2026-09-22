<?php

declare(strict_types=1);

namespace App\Http;

use App\Domain\Project\ProjectAccess;
use App\Domain\Project\Projects;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\AccessDeniedHttpException;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/**
 * The one place a request is checked against a project.
 *
 * Someone with no access gets a 404, since whether a project exists is itself
 * private. Someone whose role falls short gets a 403.
 */
final readonly class ProjectGate
{
    public function __construct(private Projects $projects) {}

    public function read(Request $request, int $projectId): ProjectAccess
    {
        $access = $this->projects->findForMember($projectId, Authenticated::id($request));
        if ($access === null) {
            throw new NotFoundHttpException('no such project');
        }

        return $access;
    }

    public function write(Request $request, int $projectId): ProjectAccess
    {
        $access = $this->read($request, $projectId);
        if (! $access->role->canWrite()) {
            throw new AccessDeniedHttpException('this project is shared with you read-only');
        }

        return $access;
    }

    public function manage(Request $request, int $projectId): ProjectAccess
    {
        $access = $this->read($request, $projectId);
        if (! $access->role->canManage()) {
            throw new AccessDeniedHttpException('only the owner can do that');
        }

        return $access;
    }
}
