<?php

declare(strict_types=1);

namespace Tests\Feature;

use App\Models\User;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Testing\TestResponse;
use Symfony\Component\HttpFoundation\Response;
use Tests\TestCase;
use ZipArchive;

final class ProjectImportApiTest extends TestCase
{
    use RefreshDatabase;

    public function test_a_zip_becomes_a_project_with_its_files(): void
    {
        $user = User::factory()->create();
        $png = "\x89PNG\r\n\x1a\n".random_bytes(64);

        $response = $this->import($user, 'Thesis', [
            'thesis.tex' => "% \\documentclass{report}\n\\documentclass{article}\n\\begin{document}\\input{chapters/one}\\end{document}\n",
            'chapters/one.tex' => 'One.',
            'chapters/standalone.tex' => '\documentclass{standalone}',
            'figures/plot.png' => $png,
            '__MACOSX/._plot.png' => 'junk',
            '../escape.tex' => 'nope',
        ])->assertCreated();

        $response->assertJsonPath('project.name', 'Thesis')
            ->assertJsonPath('project.primary', 'thesis.tex')
            ->assertJsonPath('project.role', 'owner')
            ->assertJsonPath('skipped', [['path' => '../escape.tex', 'reason' => 'invalid path']]);

        $id = $response->json('project.id');
        self::assertIsInt($id);

        $paths = $this->actingAs($user)->getJson("/api/projects/{$id}/files")->assertOk()->json('files.*.path');
        self::assertIsArray($paths);
        sort($paths);
        self::assertSame(['chapters/one.tex', 'chapters/standalone.tex', 'figures/plot.png', 'thesis.tex'], $paths);
        self::assertSame($png, $this->readBack($user, $id, 'figures/plot.png'));
    }

    public function test_main_tex_wins_and_a_shared_top_folder_is_dropped(): void
    {
        $user = User::factory()->create();

        $this->import($user, 'Paper', [
            'paper/appendix.tex' => '\documentclass{article}',
            'paper/main.tex' => '\documentclass{article}',
        ])
            ->assertCreated()
            ->assertJsonPath('project.primary', 'main.tex');
    }

    public function test_a_file_over_the_cap_is_skipped_not_fatal(): void
    {
        config(['blobs.max_file_bytes' => 8]);
        $user = User::factory()->create();

        $this->import($user, 'Paper', ['main.tex' => 'tiny', 'big.pdf' => str_repeat('x', 9)])
            ->assertCreated()
            ->assertJsonPath('skipped', [['path' => 'big.pdf', 'reason' => 'too large']]);
    }

    public function test_an_archive_over_the_cap_is_refused(): void
    {
        config(['blobs.max_import_bytes' => 16]);
        $user = User::factory()->create();

        $this->import($user, 'Paper', ['main.tex' => str_repeat('x', 64)])->assertStatus(413);
        $this->actingAs($user)->getJson('/api/projects')->assertJsonCount(0, 'projects');
    }

    public function test_something_that_is_not_a_zip_is_refused(): void
    {
        $user = User::factory()->create();

        $this->actingAs($user)
            ->call('POST', '/api/projects/import?name=Paper', server: ['CONTENT_TYPE' => 'application/zip', 'HTTP_ACCEPT' => 'application/json'], content: 'not a zip')
            ->assertUnprocessable();
    }

    public function test_a_name_is_required(): void
    {
        $user = User::factory()->create();

        $this->import($user, '  ', ['main.tex' => 'x'])->assertUnprocessable();
    }

    public function test_an_anonymous_import_is_refused(): void
    {
        $this->call('POST', '/api/projects/import?name=Paper', server: ['CONTENT_TYPE' => 'application/zip', 'HTTP_ACCEPT' => 'application/json'], content: self::zip(['main.tex' => 'x']))
            ->assertUnauthorized();
    }

    /**
     * @param  array<string, string>  $files
     * @return TestResponse<Response>
     */
    private function import(User $user, string $name, array $files): TestResponse
    {
        return $this->actingAs($user)->call(
            'POST',
            '/api/projects/import?name='.rawurlencode($name),
            server: ['CONTENT_TYPE' => 'application/zip', 'HTTP_ACCEPT' => 'application/json'],
            content: self::zip($files),
        );
    }

    /** @param  array<string, string>  $files */
    private static function zip(array $files): string
    {
        $path = tempnam(sys_get_temp_dir(), 'zip');
        self::assertIsString($path);

        $zip = new ZipArchive;
        $zip->open($path, ZipArchive::OVERWRITE);
        foreach ($files as $name => $contents) {
            $zip->addFromString($name, $contents);
        }
        $zip->close();

        $bytes = file_get_contents($path);
        unlink($path);
        self::assertIsString($bytes);

        return $bytes;
    }

    private function readBack(User $user, int $projectId, string $path): string
    {
        $url = $this->actingAs($user)
            ->getJson("/api/projects/{$projectId}/file?path={$path}")
            ->assertOk()
            ->json('url');

        self::assertIsString($url);

        $content = $this->get($url)->assertOk()->getContent();

        return $content === false ? '' : $content;
    }
}
