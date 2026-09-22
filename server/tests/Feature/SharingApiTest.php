<?php

declare(strict_types=1);

namespace Tests\Feature;

use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class SharingApiTest extends TestCase
{
    use RefreshDatabase;

    public function test_sharing_by_email_lists_the_project_for_the_new_member(): void
    {
        $owner = User::factory()->create(['name' => 'Ada']);
        $editor = User::factory()->create(['email' => 'grace@example.org']);
        $id = $this->createProject($owner, 'Thesis');

        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => 'Grace@Example.org', 'role' => 'editor'])
            ->assertCreated()
            ->assertJsonPath('member.id', $editor->id)
            ->assertJsonPath('member.role', 'editor');

        $this->actingAs($editor)
            ->getJson('/api/projects')
            ->assertOk()
            ->assertJsonPath('projects.0.id', $id)
            ->assertJsonPath('projects.0.role', 'editor')
            ->assertJsonPath('projects.0.owner.name', 'Ada');
    }

    public function test_sharing_with_an_unknown_address_or_the_owner_is_refused(): void
    {
        $owner = User::factory()->create(['email' => 'ada@example.org']);
        $id = $this->createProject($owner);

        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => 'nobody@example.org', 'role' => 'editor'])
            ->assertUnprocessable()
            ->assertJsonValidationErrors('email');

        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => 'ada@example.org', 'role' => 'viewer'])
            ->assertUnprocessable();

        $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/members", ['email' => 'ada@example.org', 'role' => 'owner'])
            ->assertUnprocessable()
            ->assertJsonValidationErrors('role');
    }

    public function test_each_role_gets_exactly_its_permissions(): void
    {
        $owner = User::factory()->create();
        $editor = User::factory()->create();
        $viewer = User::factory()->create();
        $stranger = User::factory()->create();
        $id = $this->createProject($owner);
        $this->share($owner, $id, $editor, 'editor');
        $this->share($owner, $id, $viewer, 'viewer');

        $write = fn (User $user) => $this->actingAs($user)
            ->putJson("/api/projects/{$id}/file?path=main.tex", ['text' => 'hello']);

        $write($editor)->assertOk();
        $write($viewer)->assertForbidden();
        $write($stranger)->assertNotFound();

        foreach ([$editor, $viewer] as $member) {
            $this->actingAs($member)->getJson("/api/projects/{$id}")->assertOk();
            $this->actingAs($member)->getJson("/api/projects/{$id}/files")->assertOk();
            $this->actingAs($member)->getJson("/api/projects/{$id}/file?path=main.tex")->assertOk();
            $this->actingAs($member)->getJson("/api/projects/{$id}/members")->assertOk()->assertJsonCount(0, 'invites');
            $this->actingAs($member)->deleteJson("/api/projects/{$id}")->assertForbidden();
            $this->actingAs($member)->postJson("/api/projects/{$id}/invites", ['role' => 'viewer'])->assertForbidden();
            $this->actingAs($member)
                ->postJson("/api/projects/{$id}/members", ['email' => $stranger->email, 'role' => 'viewer'])
                ->assertForbidden();
        }

        $this->actingAs($viewer)
            ->deleteJson("/api/projects/{$id}/file?path=main.tex")
            ->assertForbidden();

        $this->actingAs($stranger)->getJson("/api/projects/{$id}/members")->assertNotFound();
    }

    public function test_the_owner_changes_roles_and_removes_members(): void
    {
        $owner = User::factory()->create();
        $member = User::factory()->create();
        $id = $this->createProject($owner);
        $this->share($owner, $id, $member, 'editor');

        $this->actingAs($owner)
            ->patchJson("/api/projects/{$id}/members/{$member->id}", ['role' => 'viewer'])
            ->assertOk()
            ->assertJsonPath('member.role', 'viewer');

        $this->actingAs($member)
            ->putJson("/api/projects/{$id}/file?path=main.tex", ['text' => 'no'])
            ->assertForbidden();

        $this->actingAs($owner)->deleteJson("/api/projects/{$id}/members/{$member->id}")->assertNoContent();
        $this->actingAs($member)->getJson("/api/projects/{$id}")->assertNotFound();
        $this->actingAs($owner)->deleteJson("/api/projects/{$id}/members/{$member->id}")->assertNotFound();
    }

    public function test_a_member_can_leave_but_not_remove_others(): void
    {
        $owner = User::factory()->create();
        $first = User::factory()->create();
        $second = User::factory()->create();
        $id = $this->createProject($owner);
        $this->share($owner, $id, $first, 'editor');
        $this->share($owner, $id, $second, 'editor');

        $this->actingAs($first)->deleteJson("/api/projects/{$id}/members/{$second->id}")->assertForbidden();
        $this->actingAs($first)->deleteJson("/api/projects/{$id}/members/{$first->id}")->assertNoContent();
        $this->actingAs($first)->getJson('/api/projects')->assertOk()->assertJsonCount(0, 'projects');
    }

    public function test_an_invite_link_lets_someone_join_until_it_is_revoked(): void
    {
        $owner = User::factory()->create(['name' => 'Ada']);
        $guest = User::factory()->create();
        $late = User::factory()->create();
        $id = $this->createProject($owner, 'Thesis');

        $invite = $this->actingAs($owner)
            ->postJson("/api/projects/{$id}/invites", ['role' => 'viewer'])
            ->assertCreated()
            ->json('invite');
        self::assertIsArray($invite);
        $token = $invite['token'];
        self::assertIsString($token);
        $inviteId = $invite['id'];
        self::assertIsInt($inviteId);

        $this->actingAs($guest)
            ->getJson("/api/invites/{$token}")
            ->assertOk()
            ->assertJsonPath('invite.project.name', 'Thesis')
            ->assertJsonPath('invite.owner.name', 'Ada')
            ->assertJsonPath('invite.role', 'viewer');

        $this->actingAs($guest)
            ->postJson("/api/invites/{$token}/accept")
            ->assertOk()
            ->assertJsonPath('project.id', $id)
            ->assertJsonPath('project.role', 'viewer');

        $this->actingAs($owner)
            ->getJson("/api/projects/{$id}/members")
            ->assertOk()
            ->assertJsonPath('members.0.id', $guest->id)
            ->assertJsonPath('invites.0.id', $inviteId);

        $this->actingAs($owner)->deleteJson("/api/projects/{$id}/invites/{$inviteId}")->assertNoContent();

        $this->actingAs($late)->getJson("/api/invites/{$token}")->assertNotFound();
        $this->actingAs($late)->postJson("/api/invites/{$token}/accept")->assertNotFound();
        $this->actingAs($guest)->getJson("/api/projects/{$id}")->assertOk();
    }

    public function test_accepting_never_lowers_a_role(): void
    {
        $owner = User::factory()->create();
        $editor = User::factory()->create();
        $id = $this->createProject($owner);
        $this->share($owner, $id, $editor, 'editor');

        $viewerLink = $this->inviteToken($owner, $id, 'viewer');

        $this->actingAs($editor)
            ->postJson("/api/invites/{$viewerLink}/accept")
            ->assertOk()
            ->assertJsonPath('project.role', 'editor');

        $this->actingAs($owner)
            ->postJson("/api/invites/{$viewerLink}/accept")
            ->assertOk()
            ->assertJsonPath('project.role', 'owner');
    }

    public function test_an_editor_link_upgrades_a_viewer(): void
    {
        $owner = User::factory()->create();
        $viewer = User::factory()->create();
        $id = $this->createProject($owner);
        $this->share($owner, $id, $viewer, 'viewer');

        $editorLink = $this->inviteToken($owner, $id, 'editor');

        $this->actingAs($viewer)
            ->postJson("/api/invites/{$editorLink}/accept")
            ->assertOk()
            ->assertJsonPath('project.role', 'editor');
    }

    public function test_an_unknown_invite_is_not_found(): void
    {
        $this->actingAs(User::factory()->create())
            ->getJson('/api/invites/'.str_repeat('0', 48))
            ->assertNotFound();
    }

    public function test_deleting_a_project_removes_its_sharing(): void
    {
        $owner = User::factory()->create();
        $member = User::factory()->create();
        $id = $this->createProject($owner);
        $this->share($owner, $id, $member, 'editor');

        $this->actingAs($owner)->deleteJson("/api/projects/{$id}")->assertNoContent();

        $this->actingAs($member)->getJson('/api/projects')->assertOk()->assertJsonCount(0, 'projects');
    }

    private function createProject(User $user, string $name = 'Thesis'): int
    {
        $id = $this->actingAs($user)
            ->postJson('/api/projects', ['name' => $name])
            ->assertCreated()
            ->json('project.id');
        self::assertIsInt($id);

        return $id;
    }

    private function share(User $owner, int $projectId, User $with, string $role): void
    {
        $this->actingAs($owner)
            ->postJson("/api/projects/{$projectId}/members", ['email' => $with->email, 'role' => $role])
            ->assertCreated();
    }

    private function inviteToken(User $owner, int $projectId, string $role): string
    {
        $token = $this->actingAs($owner)
            ->postJson("/api/projects/{$projectId}/invites", ['role' => $role])
            ->assertCreated()
            ->json('invite.token');
        self::assertIsString($token);

        return $token;
    }
}
