<?php

declare(strict_types=1);

namespace Tests\Feature;

use App\Domain\Blob\BlobStore;
use App\Domain\Blob\Digest;
use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Tests\TestCase;

final class ProjectApiTest extends TestCase
{
    use RefreshDatabase;

    public function test_a_project_round_trips_through_the_api(): void
    {
        $user = User::factory()->create();

        $created = $this->actingAs($user)
            ->postJson('/api/projects', ['name' => 'Thesis'])
            ->assertCreated()
            ->json('project');

        self::assertIsArray($created);
        $id = $created['id'];
        self::assertIsInt($id);

        // The primary defaults rather than being required, because a new
        // project has nothing to point at yet.
        self::assertSame('main.tex', $created['primary']);

        $this->actingAs($user)
            ->getJson('/api/projects')
            ->assertOk()
            ->assertJsonPath('projects.0.name', 'Thesis');

        $this->actingAs($user)
            ->deleteJson("/api/projects/{$id}")
            ->assertNoContent();

        $this->actingAs($user)->getJson("/api/projects/{$id}")->assertNotFound();
    }

    public function test_a_project_is_invisible_to_everyone_but_its_owner(): void
    {
        $owner = User::factory()->create();
        $stranger = User::factory()->create();

        $id = $this->createProject($owner, 'Private');

        // Not found rather than forbidden: whether a project exists is itself
        // something only its owner should be able to learn.
        $this->actingAs($stranger)->getJson("/api/projects/{$id}")->assertNotFound();
        $this->actingAs($stranger)->deleteJson("/api/projects/{$id}")->assertNotFound();
        $this->actingAs($stranger)
            ->putJson("/api/projects/{$id}/files/main.tex", ['text' => 'mine now'])
            ->assertNotFound();
    }

    public function test_an_anonymous_request_is_refused(): void
    {
        $this->getJson('/api/projects')->assertUnauthorized();
    }

    public function test_a_file_is_stored_by_content_and_read_back(): void
    {
        $user = User::factory()->create();
        $id = $this->createProject($user);

        $body = '\documentclass{article}\begin{document}Hi\end{document}';

        $written = $this->actingAs($user)
            ->putJson("/api/projects/{$id}/files/chapters/one.tex", ['text' => $body])
            ->assertOk()
            ->json('file');

        self::assertIsArray($written);
        // A path with a slash in it is one path, not two route segments.
        self::assertSame('chapters/one.tex', $written['path']);
        self::assertSame(Digest::of($body)->value, $written['digest']);
        self::assertSame(strlen($body), $written['size']);

        $this->actingAs($user)
            ->get("/api/projects/{$id}/files/chapters/one.tex")
            ->assertOk()
            ->assertSee($body);

        $this->actingAs($user)
            ->getJson("/api/projects/{$id}/files")
            ->assertOk()
            ->assertJsonPath('files.0.path', 'chapters/one.tex');
    }

    public function test_writing_a_path_twice_replaces_it_rather_than_adding_one(): void
    {
        $user = User::factory()->create();
        $id = $this->createProject($user);

        foreach (['first', 'second'] as $body) {
            $this->actingAs($user)
                ->putJson("/api/projects/{$id}/files/main.tex", ['text' => $body])
                ->assertOk();
        }

        $files = $this->actingAs($user)->getJson("/api/projects/{$id}/files")->json('files');
        self::assertIsArray($files);
        self::assertCount(1, $files);

        $this->actingAs($user)
            ->get("/api/projects/{$id}/files/main.tex")
            ->assertSee('second');
    }

    public function test_identical_content_in_two_places_is_stored_once(): void
    {
        $user = User::factory()->create();
        $id = $this->createProject($user);

        $shared = 'the same figure caption';
        foreach (['a.tex', 'b.tex'] as $path) {
            $this->actingAs($user)
                ->putJson("/api/projects/{$id}/files/{$path}", ['text' => $shared])
                ->assertOk()
                // Content addressing is what makes this true, and the API
                // reports the address rather than hiding it.
                ->assertJsonPath('file.digest', Digest::of($shared)->value);
        }

        $blobs = $this->app->make(BlobStore::class);
        self::assertInstanceOf(BlobStore::class, $blobs);
        self::assertSame($shared, $blobs->get(Digest::of($shared)));
    }

    /**
     * Create a project and return its id, narrowed.
     *
     * `json()` hands back mixed, and interpolating that into a URL is both what
     * static analysis objects to and how a test ends up requesting
     * `/api/projects/Array`.
     */
    private function createProject(User $user, string $name = 'Thesis'): int
    {
        $id = $this->actingAs($user)
            ->postJson('/api/projects', ['name' => $name])
            ->assertCreated()
            ->json('project.id');

        self::assertIsInt($id);

        return $id;
    }

    public function test_a_path_that_climbs_out_of_the_project_is_refused(): void
    {
        $user = User::factory()->create();
        $id = $this->createProject($user);

        $this->actingAs($user)
            ->putJson("/api/projects/{$id}/files/../escaped.tex", ['text' => 'no'])
            ->assertBadRequest();
    }
}
