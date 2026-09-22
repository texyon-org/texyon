<?php

namespace App\Providers;

use App\Collab\CollabServer;
use Illuminate\Auth\Events\Logout;
use Illuminate\Support\Facades\Event;
use Illuminate\Support\ServiceProvider;

class AppServiceProvider extends ServiceProvider
{
    /**
     * Register any application services.
     */
    public function register(): void
    {
        //
    }

    /**
     * Bootstrap any application services.
     */
    public function boot(): void
    {
        // A connection outlives the session that opened it, so signing out closes it.
        Event::listen(function (Logout $event): void {
            $id = $event->user->getAuthIdentifier();
            if (is_int($id)) {
                app(CollabServer::class)->kickUser($id);
            }
        });
    }
}
