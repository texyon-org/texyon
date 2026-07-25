<?php

declare(strict_types=1);

namespace App\Domain\Project;

use InvalidArgumentException;

/**
 * A file's path within a project.
 *
 * Always relative, always normalized, and never able to leave the project it
 * belongs to. The engine enforces the same confinement on its own side — a
 * detached document root that paths are resolved under — and this is the half
 * of it that runs before anything is stored.
 */
final readonly class ProjectPath
{
    private function __construct(public string $value)
    {
    }

    /**
     * Normalize and confine a path a client supplied.
     *
     * @throws InvalidArgumentException if it cannot be made into one this
     *                                  project could contain
     */
    public static function parse(string $raw): self
    {
        if ($raw === '' || str_contains($raw, "\0")) {
            throw new InvalidArgumentException('a file path may not be empty or contain a NUL');
        }

        // Backslashes are separators to a Windows client and ordinary
        // characters to this store; treating them as separators is what stops
        // `..\..\etc` from being one long, innocent-looking file name.
        $segments = preg_split('#[/\\\\]+#', $raw);
        if ($segments === false) {
            throw new InvalidArgumentException("cannot read the path: {$raw}");
        }

        $resolved = [];
        foreach ($segments as $segment) {
            if ($segment === '' || $segment === '.') {
                continue;
            }
            if ($segment === '..') {
                // Refused rather than resolved. A path that climbs out of the
                // project is a mistake or an attack, and quietly clamping it at
                // the root would hide which.
                throw new InvalidArgumentException("a file path may not leave the project: {$raw}");
            }
            $resolved[] = $segment;
        }

        if ($resolved === []) {
            throw new InvalidArgumentException("a file path must name a file: {$raw}");
        }

        $path = implode('/', $resolved);
        if (strlen($path) > 512) {
            throw new InvalidArgumentException('a file path may not exceed 512 bytes');
        }

        return new self($path);
    }

    /** Whether this is a file TeX will be asked to typeset. */
    public function isTex(): bool
    {
        return str_ends_with(strtolower($this->value), '.tex');
    }

    public function equals(self $other): bool
    {
        return $this->value === $other->value;
    }
}
