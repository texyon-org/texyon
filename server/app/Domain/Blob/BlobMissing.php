<?php

declare(strict_types=1);

namespace App\Domain\Blob;

use RuntimeException;

/**
 * A blob the metadata says exists is not in the store.
 *
 * This is corruption rather than a normal absence: a digest only ever gets into
 * the database after the bytes it names were written.
 */
final class BlobMissing extends RuntimeException
{
    public static function for(Digest $digest): self
    {
        return new self("no blob stored for {$digest->value}");
    }
}
