<?php

declare(strict_types=1);

namespace App\Http;

use Illuminate\Http\Request;
use Symfony\Component\HttpKernel\Exception\HttpException;

/**
 * Who is making a request.
 *
 * Small, but it exists so the narrowing below is written once. The auth
 * contract types an identifier as `mixed`, because a model may be keyed by a
 * UUID as easily as by an integer, and a cast would paper over that rather
 * than check it.
 */
final readonly class Authenticated
{
    /**
     * The signed-in user's id.
     *
     * @throws HttpException if the request is not authenticated, or is
     *                       authenticated as something not keyed by an integer
     */
    public static function id(Request $request): int
    {
        $user = $request->user();
        if ($user === null) {
            // Reached only if a route escapes its auth middleware, which is a
            // routing mistake and should look like one.
            throw new HttpException(401, 'not signed in');
        }

        $id = $user->getAuthIdentifier();
        if (! is_int($id)) {
            throw new HttpException(500, 'the authenticated user is not keyed by an integer');
        }

        return $id;
    }
}
