<?php

declare(strict_types=1);

namespace App\Http\Middleware;

use App\Collab\CollabSecret;
use Closure;
use Illuminate\Http\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** Lets only the collaboration server in. Anyone else sees nothing here at all. */
final class VerifyCollabSecret
{
    /** @param Closure(Request): Response $next */
    public function handle(Request $request, Closure $next): Response
    {
        if (! CollabSecret::matches($request->header('X-Collab-Secret'))) {
            throw new NotFoundHttpException();
        }

        return $next($request);
    }
}
