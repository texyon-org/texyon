<?php

declare(strict_types=1);

namespace App\Domain\Blob;

use InvalidArgumentException;

/**
 * The content address of a stored blob.
 *
 * Blobs are named by what they contain rather than by where they came from, so
 * two projects holding the same figure hold one copy of it, and a digest is
 * safe to cache forever — nothing can change underneath it without becoming a
 * different digest.
 */
final readonly class Digest
{
    private function __construct(public string $value)
    {
    }

    /** Compute the address of some bytes. */
    public static function of(string $bytes): self
    {
        return new self(hash('sha256', $bytes));
    }

    /**
     * Accept an address that came from outside — a URL segment, a database
     * column, a client request.
     *
     * @throws InvalidArgumentException if it is not one this could have produced
     */
    public static function parse(string $value): self
    {
        if (preg_match('/^[0-9a-f]{64}$/', $value) !== 1) {
            throw new InvalidArgumentException("not a content digest: {$value}");
        }

        return new self($value);
    }

    /**
     * Where this blob lives under a store's root.
     *
     * Fanned out over two levels, because a directory holding a million
     * entries is a directory most filesystems handle badly and every listing
     * tool handles worse.
     */
    public function path(): string
    {
        return substr($this->value, 0, 2).'/'.substr($this->value, 2, 2).'/'.$this->value;
    }

    public function equals(self $other): bool
    {
        return $this->value === $other->value;
    }
}
