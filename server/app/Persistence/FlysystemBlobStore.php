<?php

declare(strict_types=1);

namespace App\Persistence;

use App\Domain\Blob\BlobMissing;
use App\Domain\Blob\BlobStore;
use App\Domain\Blob\Digest;
use Illuminate\Contracts\Filesystem\Cloud;
use Illuminate\Contracts\Filesystem\Filesystem;
use RuntimeException;

/**
 * Blobs on any disk Laravel can talk to.
 *
 * The swappable half of storage: `local` in a container with a volume, `s3`
 * against a bucket, and the application above cannot tell which — it only ever
 * sees {@see BlobStore}. Flysystem is the driver here, not the interface,
 * because its shape is a filesystem's and a content-addressed store is not one.
 */
final readonly class FlysystemBlobStore implements BlobStore
{
    public function __construct(private Filesystem $disk)
    {
    }

    public function put(string $bytes): Digest
    {
        $digest = Digest::of($bytes);
        $path = $digest->path();

        // Content addressing makes this idempotent: anything already at this
        // path is byte-for-byte what we were about to write.
        if ($this->disk->exists($path)) {
            return $digest;
        }

        if ($this->disk->put($path, $bytes) === false) {
            throw new RuntimeException("could not store blob {$digest->value}");
        }

        return $digest;
    }

    public function get(Digest $digest): string
    {
        $bytes = $this->disk->get($digest->path());
        if ($bytes === null) {
            throw BlobMissing::for($digest);
        }

        return $bytes;
    }

    public function has(Digest $digest): bool
    {
        return $this->disk->exists($digest->path());
    }

    public function temporaryUrl(Digest $digest, int $seconds): ?string
    {
        // Only some drivers can sign a URL, and which ones is an interface
        // rather than a guess about a method name. A local disk cannot, and
        // saying so is what tells the caller to serve the bytes itself.
        if (! $this->disk instanceof Cloud) {
            return null;
        }

        try {
            $url = $this->disk->temporaryUrl($digest->path(), now()->addSeconds($seconds));
        } catch (RuntimeException) {
            return null;
        }

        return $url;
    }
}
