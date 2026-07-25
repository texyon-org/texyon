<?php

declare(strict_types=1);

use App\Http\Controllers\ProjectController;
use App\Http\Controllers\ProjectFileController;
use Illuminate\Support\Facades\Route;

/*
| The editor's API.
|
| Everything is behind a session: the client is a single-page app on this same
| origin, so Sanctum's cookie guard applies rather than a bearer token.
|
| File paths are matched with `.*` because they contain slashes — `chapters/one.tex`
| is one path parameter, not two — and are confined by ProjectPath before they
| reach anything that stores them.
*/

Route::middleware('auth:sanctum')->group(function (): void {
    Route::get('/projects', [ProjectController::class, 'index']);
    Route::post('/projects', [ProjectController::class, 'store']);
    Route::get('/projects/{project}', [ProjectController::class, 'show']);
    Route::delete('/projects/{project}', [ProjectController::class, 'destroy']);

    Route::get('/projects/{project}/files', [ProjectFileController::class, 'index']);
    Route::get('/projects/{project}/files/{path}', [ProjectFileController::class, 'show'])
        ->where('path', '.*');
    Route::put('/projects/{project}/files/{path}', [ProjectFileController::class, 'put'])
        ->where('path', '.*');
    Route::delete('/projects/{project}/files/{path}', [ProjectFileController::class, 'destroy'])
        ->where('path', '.*');
});
