<?php

declare(strict_types=1);

namespace App\Domain\Blob;

/**
 * Where a project's bytes live.
 *
 * Deliberately not a filesystem. S3 has no rename, no partial write and no
 * directories, and an interface shaped like POSIX would either lie about that
 * or leak it. What a project actually needs is narrower: store some bytes, get
 * them back by what they are, and hand a client a way to fetch them without
 * the bytes passing through PHP.
 */
interface BlobStore
{
    /**
     * Store bytes and return their address.
     *
     * Storing the same bytes twice is not an error and costs nothing the second
     * time: the address is already taken by a copy that is identical.
     */
    public function put(string $bytes): Digest;

    /**
     * Read a blob back.
     *
     * @throws BlobMissing if the store does not hold it
     */
    public function get(Digest $digest): string;

    public function has(Digest $digest): bool;

    /**
     * A URL a browser can fetch the blob from directly, if the backing store
     * can issue one.
     *
     * Returns null for stores that cannot — a local directory has no way to
     * grant time-limited access to one file — and the caller then has to serve
     * the bytes itself.
     */
    public function temporaryUrl(Digest $digest, int $seconds): ?string;
}
