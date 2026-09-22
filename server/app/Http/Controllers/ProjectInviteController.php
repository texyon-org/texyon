<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Project\Sharing;
use App\Http\Authenticated;
use App\Http\Presenters\ProjectPresenter;
use App\Http\ProjectGate;
use App\Http\Requests\RoleRequest;
use Dedoc\Scramble\Attributes\Response as ResponseShape;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** Links the owner hands out to let people join a project. */
final readonly class ProjectInviteController
{
    public function __construct(
        private Sharing $sharing,
        private ProjectGate $gate,
    ) {}

    /** Create a link. The token is returned this once and never again. */
    #[ResponseShape(201, type: 'array{invite: array{id: int, role: string, createdAt: string, token: string}}')]
    public function store(RoleRequest $request, int $project): JsonResponse
    {
        $access = $this->gate->manage($request, $project);

        $token = bin2hex(random_bytes(24));
        $invite = $this->sharing->createInvite(
            $access->project,
            $request->role(),
            $token,
            Authenticated::id($request),
        );

        return new JsonResponse([
            'invite' => [...ProjectPresenter::invite($invite), 'token' => $token],
        ], 201);
    }

    /** Revoke a link. People who already joined through it keep their access. */
    public function destroy(Request $request, int $project, int $invite): Response
    {
        $access = $this->gate->manage($request, $project);

        if (! $this->sharing->deleteInvite($access->project, $invite)) {
            throw new NotFoundHttpException('no such invite');
        }

        return response()->noContent();
    }
}
