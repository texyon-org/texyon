<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Domain\Blob\BlobMissing;
use App\Domain\Blob\BlobStore;
use App\Domain\Blob\Digest;
use Dedoc\Scramble\Attributes\ExcludeRouteFromDocs;
use Illuminate\Http\Response;
use InvalidArgumentException;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** Raw blob bytes, for stores that cannot sign a URL of their own. */
final readonly class BlobController
{
    public function __construct(private BlobStore $blobs) {}

    // Reached via the signed URL the file endpoint returns. The signature is
    // the authorization.
    #[ExcludeRouteFromDocs]
    public function read(string $digest): Response
    {
        try {
            $parsed = Digest::parse($digest);
            $bytes = $this->blobs->get($parsed);
        } catch (InvalidArgumentException|BlobMissing) {
            throw new NotFoundHttpException('no such blob');
        }

        return new Response($bytes, 200, [
            'Content-Type' => 'application/octet-stream',
            'ETag' => '"'.$parsed->value.'"',
            'Cache-Control' => 'private, max-age=31536000, immutable',
        ]);
    }
}
