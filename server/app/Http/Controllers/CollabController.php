<?php

declare(strict_types=1);

namespace App\Http\Controllers;

use App\Collab\CollabToken;
use App\Domain\Blob\BlobStore;
use App\Domain\Blob\Digest;
use App\Domain\Collab\Documents;
use App\Http\Authenticated;
use App\Http\OpenApi\RawBody;
use App\Http\ProjectGate;
use App\Models\User;
use Dedoc\Scramble\Attributes\Response as ResponseShape;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Config;
use Illuminate\Support\Facades\URL;
use InvalidArgumentException;
use Symfony\Component\HttpKernel\Exception\HttpException;
use Symfony\Component\HttpKernel\Exception\NotFoundHttpException;

/** What the editor needs to join a project's shared document. */
final readonly class CollabController
{
    public function __construct(
        private ProjectGate $gate,
        private BlobStore $blobs,
        private Documents $documents,
    ) {}

    /** A pass to the project's document, good for about a minute. Fetched again on every reconnect. */
    public function token(Request $request, int $project): JsonResponse
    {
        $access = $this->gate->read($request, $project);
        $user = $request->user();
        $name = $user instanceof User ? $user->name : '';

        return new JsonResponse(CollabToken::issue($access, Authenticated::id($request), $name));
    }

    /** Where to fetch bytes the project's document names, before or after they became a file. */
    public function blobUrl(Request $request, int $project, string $digest): JsonResponse
    {
        $access = $this->gate->read($request, $project);
        try {
            $parsed = Digest::parse($digest);
        } catch (InvalidArgumentException) {
            throw new NotFoundHttpException('no such blob');
        }
        if (! $this->documents->ownsBlob($access->project, $parsed)) {
            throw new NotFoundHttpException('no such blob');
        }

        $lifetime = Config::integer('blobs.url_lifetime');
        $url = $this->blobs->temporaryUrl($parsed, $lifetime)
            ?? URL::temporarySignedRoute('blobs.read', now()->addSeconds($lifetime), ['digest' => $parsed->value], absolute: false);

        return new JsonResponse(['url' => $url]);
    }

    /** Upload bytes the document can then name, as a binary file. */
    #[RawBody]
    #[ResponseShape(200, type: 'array{digest: string, size: int}')]
    public function blob(Request $request, int $project): JsonResponse
    {
        $access = $this->gate->write($request, $project);
        $contents = $request->getContent();

        if (strlen($contents) > Config::integer('blobs.max_file_bytes')) {
            throw new HttpException(413, 'the file is larger than the server accepts');
        }

        $digest = $this->blobs->put($contents);
        $this->documents->recordBlob($access->project, $digest);

        return new JsonResponse(['digest' => $digest->value, 'size' => strlen($contents)]);
    }
}
