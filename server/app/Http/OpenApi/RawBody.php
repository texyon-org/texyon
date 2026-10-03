<?php

declare(strict_types=1);

namespace App\Http\OpenApi;

use Attribute;

/** The action reads its request body raw, as bytes of this media type. */
#[Attribute(Attribute::TARGET_METHOD)]
final readonly class RawBody
{
    public function __construct(public string $mediaType = 'application/octet-stream') {}
}
