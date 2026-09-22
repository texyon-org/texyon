<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Project\Invite;
use App\Domain\Project\Sharing;
use App\Http\Authenticated;
use App\Http\Presenters\ProjectPresenter;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** An invite link, as seen by the person who opened it. */
final readonly class InviteController
{
    public function __construct(private Sharing $sharing) {}

    /** What joining would give: which project, whose, and in which role. */
    public function show(Request $request, string $token): JsonResponse
    {
        $invite = $this->find($token);

        return new JsonResponse([
            'invite' => [
                'role' => $invite->role->value,
                'project' => ['id' => $invite->project->id, 'name' => $invite->project->name],
                'owner' => ['name' => $invite->project->ownerName],
            ],
        ]);
    }

    /** Join. Opening a link again, or one for a lesser role, changes nothing. */
    public function accept(Request $request, string $token): JsonResponse
    {
        $access = $this->sharing->accept($this->find($token), Authenticated::id($request));

        return new JsonResponse(['project' => ProjectPresenter::project($access)]);
    }

    private function find(string $token): Invite
    {
        $invite = $this->sharing->findInvite($token);
        if ($invite === null) {
            throw new NotFoundHttpException('this invite link is not valid');
        }

        return $invite;
    }
}
