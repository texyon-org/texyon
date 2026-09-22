<?php

declare(strict_types=1);

namespace App\Providers;

use App\Domain\Blob\BlobStore;
use App\Domain\Collab\Documents;
use App\Domain\Project\Projects;
use App\Domain\Project\Sharing;
use App\Persistence\EloquentDocuments;
use App\Persistence\EloquentProjects;
use App\Persistence\EloquentSharing;
use App\Persistence\FlysystemBlobStore;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\Storage;
use Illuminate\Support\ServiceProvider;

/**
 * Binds the two things the application stores through.
 *
 * Which disk blobs land on is configuration, not code: `local` for a container
 * with a volume, `s3` for a bucket. Nothing above these bindings changes.
 */
final class StorageServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->app->singleton(BlobStore::class, static function (): BlobStore {
            // Config::string rather than config(): the untyped helper returns
            // mixed, and a disk name that silently became null would fail at
            // the first upload rather than at boot.
            return new FlysystemBlobStore(Storage::disk(Config::string('blobs.disk')));
        });

        $this->app->singleton(Projects::class, EloquentProjects::class);
        $this->app->singleton(Sharing::class, EloquentSharing::class);
        $this->app->singleton(Documents::class, EloquentDocuments::class);
    }
}
