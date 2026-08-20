<?php

declare(strict_types=1);

use App\Http\Controllers\AccountController;
use App\Http\Controllers\BlobController;
use App\Http\Controllers\ProjectController;
use App\Http\Controllers\ProjectFileController;
use Illuminate\Support\Facades\Route;

// The editor's API, behind Sanctum's cookie guard. Exported to openapi.json,
// which the client's bindings are generated from. CI fails on drift.
Route::middleware('auth:sanctum')->group(function (): void {
    Route::get('/account', [AccountController::class, 'show']);
    // Deletion wants the password re-confirmed, the same 423 dance Fortify uses.
    Route::delete('/account', [AccountController::class, 'destroy'])->middleware('password.confirm');

    Route::get('/projects', [ProjectController::class, 'index']);
    Route::post('/projects', [ProjectController::class, 'store']);
    Route::get('/projects/{project}', [ProjectController::class, 'show']);
    Route::delete('/projects/{project}', [ProjectController::class, 'destroy']);

    Route::get('/projects/{project}/files', [ProjectFileController::class, 'index']);
    Route::get('/projects/{project}/file', [ProjectFileController::class, 'show']);
    Route::put('/projects/{project}/file', [ProjectFileController::class, 'put']);
    Route::put('/projects/{project}/file/bytes', [ProjectFileController::class, 'putBytes']);
    Route::delete('/projects/{project}/file', [ProjectFileController::class, 'destroy']);
});

// Not behind the session. The signature is the authorization, like an object
// store's temporary URL.
Route::get('/blobs/{digest}', [BlobController::class, 'read'])
    ->middleware('signed:relative')
    ->name('blobs.read');
