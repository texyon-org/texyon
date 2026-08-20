<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Project\Projects;
use App\Http\Authenticated;
use App\Models\User;
use Dedoc\Scramble\Attributes\Response as ResponseShape;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Http\Response;
use Illuminate\Support\Facades\Auth;
use Symfony\Component\HttpKernel\Exception\HttpException;

/** The signed-in account itself. */
final readonly class AccountController
{
    public function __construct(private Projects $projects) {}

    #[ResponseShape(200, type: 'array{user: array{name: string, email: string, twoFactorEnabled: bool}}')]
    public function show(Request $request): JsonResponse
    {
        $user = $this->user($request);

        return new JsonResponse([
            'user' => [
                'name' => $user->name,
                'email' => $user->email,
                'twoFactorEnabled' => $user->hasEnabledTwoFactorAuthentication(),
            ],
        ]);
    }

    public function destroy(Request $request): Response
    {
        $user = $this->user($request);

        // Through the repository, so blob references are released and not just rows.
        foreach ($this->projects->ownedBy(Authenticated::id($request)) as $project) {
            $this->projects->delete($project);
        }

        // Logging out saves the user (remember token), so it must precede the delete.
        if ($request->hasSession()) {
            Auth::guard('web')->logout();
            $request->session()->invalidate();
            $request->session()->regenerateToken();
        }
        $user->delete();

        return response()->noContent();
    }

    private function user(Request $request): User
    {
        $user = $request->user();
        if (! $user instanceof User) {
            // Reached only if a route escapes its auth middleware.
            throw new HttpException(401, 'not signed in');
        }

        return $user;
    }
}
