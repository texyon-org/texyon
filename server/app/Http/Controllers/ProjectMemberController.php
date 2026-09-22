<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Collab\CollabServer;
use App\Domain\Project\Sharing;
use App\Http\Authenticated;
use App\Http\Presenters\ProjectPresenter;
use App\Http\ProjectGate;
use App\Http\Requests\RoleRequest;
use App\Http\Requests\ShareProjectRequest;
use Dedoc\Scramble\Attributes\Response as ResponseShape;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Validation\ValidationException;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** Who a project is shared with. Only the owner changes it, but anyone in it can look. */
final readonly class ProjectMemberController
{
    public function __construct(
        private Sharing $sharing,
        private ProjectGate $gate,
        private CollabServer $collab,
    ) {}

    /** The owner, everyone else with access, and for the owner the open invite links. */
    #[ResponseShape(200, type: 'array{owner: '.ProjectPresenter::MEMBER.', members: list<'.ProjectPresenter::MEMBER.'>, invites: list<'.ProjectPresenter::INVITE.'>}')]
    public function index(Request $request, int $project): JsonResponse
    {
        $access = $this->gate->read($request, $project);
        $invites = $access->role->canManage() ? $this->sharing->invites($access->project) : [];

        return new JsonResponse([
            'owner' => ProjectPresenter::member($this->sharing->owner($access->project)),
            'members' => array_map(ProjectPresenter::member(...), $this->sharing->members($access->project)),
            'invites' => array_map(ProjectPresenter::invite(...), $invites),
        ]);
    }

    /** Share with an existing account by email address, or change the role of someone already in. */
    public function store(ShareProjectRequest $request, int $project): JsonResponse
    {
        $access = $this->gate->manage($request, $project);

        $member = $this->sharing->share($access->project, $request->email(), $request->role());
        if ($member === null) {
            throw ValidationException::withMessages([
                'email' => 'No other account uses that address.',
            ]);
        }

        return new JsonResponse(['member' => ProjectPresenter::member($member)], 201);
    }

    public function update(RoleRequest $request, int $project, int $user): JsonResponse
    {
        $access = $this->gate->manage($request, $project);

        $member = $this->sharing->setRole($access->project, $user, $request->role());
        if ($member === null) {
            throw new NotFoundHttpException('no such member');
        }
        // Their open editor reconnects and picks the new role up from a fresh token.
        $this->collab->kickUser($user, $project);

        return new JsonResponse(['member' => ProjectPresenter::member($member)]);
    }

    /** Take someone out of a project. Members may also remove themselves. */
    public function destroy(Request $request, int $project, int $user): Response
    {
        $access = $user === Authenticated::id($request)
            ? $this->gate->read($request, $project)
            : $this->gate->manage($request, $project);

        if (! $this->sharing->unshare($access->project, $user)) {
            throw new NotFoundHttpException('no such member');
        }
        $this->collab->kickUser($user, $project);

        return response()->noContent();
    }
}
