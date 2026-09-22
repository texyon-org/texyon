<?php

declare(strict_types=1);

use App\Http\Controllers\AccountController;
use App\Http\Controllers\BlobController;
use App\Http\Controllers\CollabController;
use App\Http\Controllers\Internal\CollabDocumentController;
use App\Http\Controllers\InviteController;
use App\Http\Controllers\ProjectController;
use App\Http\Controllers\ProjectFileController;
use App\Http\Controllers\ProjectInviteController;
use App\Http\Controllers\ProjectMemberController;
use App\Http\Middleware\VerifyCollabSecret;
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

    Route::get('/projects/{project}/members', [ProjectMemberController::class, 'index']);
    Route::post('/projects/{project}/members', [ProjectMemberController::class, 'store']);
    Route::patch('/projects/{project}/members/{user}', [ProjectMemberController::class, 'update']);
    Route::delete('/projects/{project}/members/{user}', [ProjectMemberController::class, 'destroy']);
    Route::post('/projects/{project}/invites', [ProjectInviteController::class, 'store']);
    Route::delete('/projects/{project}/invites/{invite}', [ProjectInviteController::class, 'destroy']);

    Route::post('/projects/{project}/collab-token', [CollabController::class, 'token']);
    Route::put('/projects/{project}/blobs', [CollabController::class, 'blob']);

    Route::get('/invites/{token}', [InviteController::class, 'show']);
    Route::post('/invites/{token}/accept', [InviteController::class, 'accept']);
});

// The collaboration server's own door, keyed by a secret derived from APP_KEY.
Route::middleware(VerifyCollabSecret::class)->prefix('internal/collab')->group(function (): void {
    Route::get('/projects/{project}', [CollabDocumentController::class, 'show']);
    Route::put('/projects/{project}', [CollabDocumentController::class, 'put']);
});

// Not behind the session. The signature is the authorization, like an object
// store's temporary URL.
Route::get('/blobs/{digest}', [BlobController::class, 'read'])
    ->middleware('signed:relative')
    ->name('blobs.read');
