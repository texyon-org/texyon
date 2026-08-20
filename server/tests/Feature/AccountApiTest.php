<?php

declare(strict_types=1);

namespace Tests\Feature;

use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class AccountApiTest extends TestCase
{
    use RefreshDatabase;

    public function test_the_account_is_described(): void
    {
        $user = User::factory()->create(['name' => 'Ada', 'email' => 'ada@example.org']);

        $this->actingAs($user)
            ->getJson('/api/account')
            ->assertOk()
            ->assertExactJson([
                'user' => [
                    'name' => 'Ada',
                    'email' => 'ada@example.org',
                    'twoFactorEnabled' => false,
                ],
            ]);
    }

    public function test_deleting_the_account_requires_a_confirmed_password(): void
    {
        $user = User::factory()->create();

        // Stateful like the SPA, password.confirm needs the session.
        $this->actingAs($user)
            ->withHeader('Origin', $this->statefulOrigin())
            ->deleteJson('/api/account')
            ->assertStatus(423);

        self::assertNotNull(User::query()->find($user->id));
    }

    public function test_deleting_the_account_removes_it_and_its_projects(): void
    {
        $user = User::factory()->create();

        $projectId = $this->actingAs($user)
            ->postJson('/api/projects', ['name' => 'Doomed'])
            ->assertCreated()
            ->json('project.id');
        self::assertIsInt($projectId);

        $this->actingAs($user)
            ->withHeader('Origin', $this->statefulOrigin())
            ->withSession(['auth.password_confirmed_at' => time()])
            ->deleteJson('/api/account')
            ->assertNoContent();

        self::assertNull(User::query()->find($user->id));
        $this->assertDatabaseMissing('projects', ['id' => $projectId]);
    }

    public function test_an_anonymous_request_is_refused(): void
    {
        $this->getJson('/api/account')->assertUnauthorized();
    }

    /** An Origin Sanctum considers first-party. */
    private function statefulOrigin(): string
    {
        $stateful = config('sanctum.stateful');
        self::assertIsArray($stateful);
        $domain = $stateful[0] ?? null;
        self::assertIsString($domain);

        return 'http://'.$domain;
    }
}
