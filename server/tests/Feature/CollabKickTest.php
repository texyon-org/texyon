<?php

declare(strict_types=1);

namespace Tests\Feature;

use App\Collab\CollabSecret;
use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Http\Client\Request;
use Illuminate\Support\Facades\Http;
use Tests\TestCase;

final class CollabKickTest extends TestCase
{
    use RefreshDatabase;

    protected function setUp(): void
    {
        parent::setUp();
        Http::fake(['*/collab/internal/kick' => Http::response(['closed' => 1])]);
    }

    public function test_a_role_change_or_removal_disconnects_that_member(): void
    {
        $owner = User::factory()->create();
        $member = User::factory()->create();
        $id = $this->createProject($owner);
        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => $member->email, 'role' => 'editor'])
            ->assertCreated();

        $this->actingAs($owner)->patchJson("/api/projects/{$id}/members/{$member->id}", ['role' => 'viewer'])->assertOk();
        $this->assertKicked(['user' => $member->id, 'project' => $id]);

        $this->actingAs($owner)->deleteJson("/api/projects/{$id}/members/{$member->id}")->assertNoContent();
        $this->assertKicked(['user' => $member->id, 'project' => $id], 2);
    }

    public function test_deleting_a_project_disconnects_everyone_in_it(): void
    {
        $owner = User::factory()->create();
        $id = $this->createProject($owner);

        $this->actingAs($owner)->deleteJson("/api/projects/{$id}")->assertNoContent();

        $this->assertKicked(['project' => $id]);
    }

    public function test_signing_out_disconnects_that_user(): void
    {
        $user = User::factory()->create();

        $this->actingAs($user)->postJson('/logout')->assertNoContent();

        $this->assertKicked(['user' => $user->id]);
    }

    public function test_an_unreachable_collab_server_does_not_fail_the_request(): void
    {
        Http::fake(['*/collab/internal/kick' => fn () => throw new \Illuminate\Http\Client\ConnectionException('refused')]);
        $owner = User::factory()->create();
        $id = $this->createProject($owner);

        $this->actingAs($owner)->deleteJson("/api/projects/{$id}")->assertNoContent();
    }

    /** @param array<string, int> $body */
    private function assertKicked(array $body, int $times = 1): void
    {
        Http::assertSentCount($times);
        Http::assertSent(fn (Request $request): bool => $request->method() === 'POST'
            && str_ends_with($request->url(), '/collab/internal/kick')
            && $request->header('X-Collab-Secret') === [CollabSecret::hex()]
            && $request->data() === $body);
    }

    private function createProject(User $user): int
    {
        $id = $this->actingAs($user)->postJson('/api/projects', ['name' => 'Thesis'])->assertCreated()->json('project.id');
        self::assertIsInt($id);

        return $id;
    }
}
