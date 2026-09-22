<?php

declare(strict_types=1);

namespace Tests\Feature;

use App\Collab\CollabSecret;
use App\Domain\Blob\Digest;
use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Testing\TestResponse;
use Tests\TestCase;

final class CollabApiTest extends TestCase
{
    use RefreshDatabase;

    public function test_a_member_gets_a_signed_token_naming_their_role(): void
    {
        $owner = User::factory()->create();
        $viewer = User::factory()->create(['name' => 'Grace']);
        $id = $this->createProject($owner);
        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => $viewer->email, 'role' => 'viewer'])
            ->assertCreated();

        $token = $this->actingAs($viewer)->postJson("/api/projects/{$id}/collab-token")->assertOk()->json('token');
        self::assertIsString($token);

        [$body, $signature] = explode('.', $token);
        $expected = rtrim(strtr(base64_encode(hash_hmac('sha256', $body, CollabSecret::bytes(), true)), '+/', '-_'), '=');
        self::assertSame($expected, $signature);

        $claims = json_decode((string) base64_decode(strtr($body, '-_', '+/'), true), true);
        self::assertIsArray($claims);
        self::assertSame($viewer->id, $claims['uid']);
        self::assertSame($id, $claims['pid']);
        self::assertSame('viewer', $claims['role']);
        self::assertSame('Grace', $claims['name']);
        self::assertIsInt($claims['exp']);
        self::assertGreaterThan(time(), $claims['exp']);

        $this->actingAs(User::factory()->create())->postJson("/api/projects/{$id}/collab-token")->assertNotFound();
    }

    public function test_the_internal_endpoints_are_invisible_without_the_secret(): void
    {
        $id = $this->createProject(User::factory()->create());

        $this->getJson("/api/internal/collab/projects/{$id}")->assertNotFound();
        $this->getJson("/api/internal/collab/projects/{$id}", ['X-Collab-Secret' => str_repeat('0', 64)])->assertNotFound();
        $this->internal('GET', $id)->assertOk();
        $this->internal('GET', 999)->assertNotFound();
    }

    public function test_a_fresh_document_is_seeded_from_text_and_binary_files(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);
        $this->actingAs($owner)->putJson("/api/projects/{$id}/file?path=main.tex", ['text' => "Hello.\n"])->assertOk();
        $png = "\x89PNG\r\n\x1a\n\0\0";
        $this->actingAs($owner)
            ->call('PUT', "/api/projects/{$id}/file/bytes?path=fig.png", server: ['CONTENT_TYPE' => 'application/octet-stream'], content: $png)
            ->assertOk();

        $this->internal('GET', $id)
            ->assertOk()
            ->assertJsonPath('state', null)
            ->assertJsonPath('files.0.path', 'main.tex')
            ->assertJsonPath('files.0.text', "Hello.\n")
            ->assertJsonPath('binaries.0.path', 'fig.png')
            ->assertJsonPath('binaries.0.digest', Digest::of($png)->value);
    }

    public function test_only_the_first_seed_is_kept(): void
    {
        $id = $this->createProject(User::factory()->create());

        $this->internal('PUT', $id, $this->document('first'), '?create=1')->assertOk();
        $this->internal('PUT', $id, $this->document('second'), '?create=1')->assertStatus(409);

        $this->internal('GET', $id)
            ->assertOk()
            ->assertJsonPath('state', base64_encode('first'))
            ->assertJsonPath('files', []);
    }

    public function test_storing_a_document_brings_the_files_in_line_with_it(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);
        $this->actingAs($owner)->putJson("/api/projects/{$id}/file?path=old.tex", ['text' => 'gone soon'])->assertOk();
        $this->actingAs($owner)->putJson("/api/projects/{$id}/file?path=main.tex", ['text' => 'before'])->assertOk();

        $mine = 'bytes this project uploaded';
        $this->actingAs($owner)
            ->call('PUT', "/api/projects/{$id}/blobs", server: ['CONTENT_TYPE' => 'application/octet-stream'], content: $mine)
            ->assertOk()
            ->assertJsonPath('digest', Digest::of($mine)->value);

        $other = $this->createProject(User::factory()->create());
        $theirs = 'someone else\'s figure';
        $this->internal('PUT', $other, $this->document('x', [['path' => 'a.tex', 'text' => $theirs]]))->assertOk();

        $this->internal('PUT', $id, $this->document('s', [
            ['path' => 'main.tex', 'text' => "  after\n"],
            ['path' => 'ch/new.tex', 'text' => ''],
            ['path' => '../escape.tex', 'text' => 'no'],
        ], [
            ['path' => 'fig.bin', 'digest' => Digest::of($mine)->value, 'size' => strlen($mine)],
            ['path' => 'stolen.bin', 'digest' => Digest::of($theirs)->value, 'size' => strlen($theirs)],
        ]))->assertOk();

        $paths = $this->actingAs($owner)->getJson("/api/projects/{$id}/files")->assertOk()->json('files.*.path');
        self::assertSame(['ch/new.tex', 'fig.bin', 'main.tex'], $paths);
        self::assertSame("  after\n", $this->readBack($owner, $id, 'main.tex'));
        self::assertSame('', $this->readBack($owner, $id, 'ch/new.tex'));
    }

    public function test_files_are_not_written_directly_once_a_project_is_live(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);
        $this->internal('PUT', $id, $this->document('s', [['path' => 'main.tex', 'text' => 'live']]), '?create=1')->assertOk();

        $this->actingAs($owner)->putJson("/api/projects/{$id}/file?path=main.tex", ['text' => 'stale'])->assertStatus(409);
        $this->actingAs($owner)->deleteJson("/api/projects/{$id}/file?path=main.tex")->assertStatus(409);
        self::assertSame('live', $this->readBack($owner, $id, 'main.tex'));
    }

    public function test_a_deleted_project_cannot_be_stored_to(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);
        $this->actingAs($owner)->deleteJson("/api/projects/{$id}")->assertNoContent();

        $this->internal('PUT', $id, $this->document('late'))->assertNotFound();
    }

    public function test_only_writers_upload_blobs(): void
    {
        $owner = User::factory()->create();
        $viewer = User::factory()->create();
        $id = $this->createProject($owner);
        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => $viewer->email, 'role' => 'viewer'])
            ->assertCreated();

        $this->actingAs($viewer)
            ->call('PUT', "/api/projects/{$id}/blobs", server: ['CONTENT_TYPE' => 'application/octet-stream'], content: 'x')
            ->assertForbidden();
    }

    public function test_an_uploaded_blob_is_readable_by_members_only(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);
        $bytes = "\x89PNG figure";
        $this->actingAs($owner)
            ->call('PUT', "/api/projects/{$id}/blobs", server: ['CONTENT_TYPE' => 'application/octet-stream'], content: $bytes)
            ->assertOk();
        $digest = Digest::of($bytes)->value;

        $url = $this->actingAs($owner)->getJson("/api/projects/{$id}/blobs/{$digest}")->assertOk()->json('url');
        self::assertIsString($url);
        self::assertSame($bytes, $this->get($url)->assertOk()->getContent());

        $other = $this->createProject($owner);
        $this->actingAs($owner)->getJson("/api/projects/{$other}/blobs/{$digest}")->assertNotFound();
        $this->actingAs(User::factory()->create())->getJson("/api/projects/{$id}/blobs/{$digest}")->assertNotFound();
        $this->actingAs($owner)->getJson("/api/projects/{$id}/blobs/nothex")->assertNotFound();
    }

    public function test_whitespace_around_a_file_survives_the_text_endpoint(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);

        $this->actingAs($owner)->putJson("/api/projects/{$id}/file?path=main.tex", ['text' => "\n  body\n\n"])->assertOk();

        self::assertSame("\n  body\n\n", $this->readBack($owner, $id, 'main.tex'));
    }

    /**
     * @param  list<array{path: string, text: string}>  $files
     * @param  list<array{path: string, digest: string, size: int}>  $binaries
     * @return array<string, mixed>
     */
    private function document(string $state, array $files = [], array $binaries = []): array
    {
        return ['state' => base64_encode($state), 'epoch' => 'e1', 'files' => $files, 'binaries' => $binaries];
    }

    /**
     * @param  array<string, mixed>  $body
     * @return TestResponse<\Symfony\Component\HttpFoundation\Response>
     */
    private function internal(string $method, int $projectId, array $body = [], string $query = ''): TestResponse
    {
        return $this->json($method, "/api/internal/collab/projects/{$projectId}{$query}", $body, [
            'X-Collab-Secret' => CollabSecret::hex(),
        ]);
    }

    private function createProject(User $user): int
    {
        $id = $this->actingAs($user)->postJson('/api/projects', ['name' => 'Thesis'])->assertCreated()->json('project.id');
        self::assertIsInt($id);

        return $id;
    }

    private function readBack(User $user, int $projectId, string $path): string
    {
        $url = $this->actingAs($user)->getJson("/api/projects/{$projectId}/file?path={$path}")->assertOk()->json('url');
        self::assertIsString($url);
        $content = $this->get($url)->assertOk()->getContent();

        return $content === false ? '' : $content;
    }
}
