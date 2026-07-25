<?php

declare(strict_types=1);

namespace Tests\Unit;

use App\Domain\Project\ProjectPath;
use InvalidArgumentException;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

final class ProjectPathTest extends TestCase
{
    #[DataProvider('normalizations')]
    public function test_a_path_is_normalized_to_one_spelling(string $raw, string $expected): void
    {
        $this->assertSame($expected, ProjectPath::parse($raw)->value);
    }

    /** @return iterable<string, array{string, string}> */
    public static function normalizations(): iterable
    {
        yield 'plain' => ['main.tex', 'main.tex'];
        yield 'nested' => ['chapters/one.tex', 'chapters/one.tex'];
        yield 'leading slash' => ['/main.tex', 'main.tex'];
        yield 'redundant separators' => ['chapters//one.tex', 'chapters/one.tex'];
        yield 'current directory' => ['./chapters/./one.tex', 'chapters/one.tex'];
        // A client on Windows sends these, and treating them as ordinary
        // characters is what would let `..\..\etc` through as a file name.
        yield 'backslashes' => ['chapters\\one.tex', 'chapters/one.tex'];
    }

    #[DataProvider('refusals')]
    public function test_a_path_that_could_leave_the_project_is_refused(string $raw): void
    {
        $this->expectException(InvalidArgumentException::class);
        ProjectPath::parse($raw);
    }

    /** @return iterable<string, array{string}> */
    public static function refusals(): iterable
    {
        yield 'parent' => ['../secrets.tex'];
        yield 'parent in the middle' => ['chapters/../../secrets.tex'];
        // Refused rather than clamped: climbing out is a mistake or an attack,
        // and silently pinning it at the root would hide which.
        yield 'parent that would resolve inside' => ['chapters/../main.tex'];
        yield 'windows parent' => ['..\\secrets.tex'];
        yield 'empty' => [''];
        yield 'only separators' => ['///'];
        yield 'only dots' => ['.'];
        yield 'nul byte' => ["main\0.tex"];
        yield 'absurdly long' => [str_repeat('a', 513)];
    }

    public function test_a_tex_file_is_recognised_whatever_its_case(): void
    {
        $this->assertTrue(ProjectPath::parse('main.tex')->isTex());
        $this->assertTrue(ProjectPath::parse('Main.TeX')->isTex());
        $this->assertFalse(ProjectPath::parse('figure.png')->isTex());
        $this->assertFalse(ProjectPath::parse('texture.png')->isTex());
    }
}
