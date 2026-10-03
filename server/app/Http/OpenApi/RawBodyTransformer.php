<?php

declare(strict_types=1);

namespace App\Http\OpenApi;

use Dedoc\Scramble\Contracts\OperationTransformer;
use Dedoc\Scramble\Support\Generator\Operation;
use Dedoc\Scramble\Support\Generator\RequestBodyObject;
use Dedoc\Scramble\Support\Generator\Schema;
use Dedoc\Scramble\Support\Generator\Types\StringType;
use Dedoc\Scramble\Support\RouteInfo;

/** Scramble only derives a body from validation rules, so a #[RawBody] action describes its own. */
final class RawBodyTransformer implements OperationTransformer
{
    public function handle(Operation $operation, RouteInfo $routeInfo): void
    {
        $attribute = $routeInfo->reflectionAction()?->getAttributes(RawBody::class)[0] ?? null;
        if ($attribute === null) {
            return;
        }

        // Scramble's fluent setters are untyped, so no chaining.
        $body = new RequestBodyObject;
        $bytes = new StringType;
        $bytes->format('binary');
        /** @var Schema $schema */
        $schema = Schema::fromType($bytes);
        $body->setContent($attribute->newInstance()->mediaType, $schema);
        $body->required();
        $operation->addRequestBodyObject($body);
    }
}
