<?php

declare(strict_types=1);

namespace Tests\Unit;

use App\Domain\Blob\Digest;
use InvalidArgumentException;
use PHPUnit\Framework\TestCase;

final class DigestTest extends TestCase
{
    public function test_the_same_bytes_have_the_same_address(): void
    {
        $this->assertTrue(Digest::of('hello')->equals(Digest::of('hello')));
        $this->assertFalse(Digest::of('hello')->equals(Digest::of('hellp')));
    }

    public function test_a_digest_fans_out_over_two_levels(): void
    {
        $digest = Digest::of('hello');
        $value = $digest->value;

        $this->assertSame(
            substr($value, 0, 2).'/'.substr($value, 2, 2).'/'.$value,
            $digest->path(),
        );
    }

    public function test_something_that_is_not_an_address_is_refused(): void
    {
        // A digest reaches parse() from a URL segment or a database column, so
        // anything that is not one has come from somewhere it should not have.
        foreach (['', 'nope', str_repeat('g', 64), str_repeat('a', 63), strtoupper(str_repeat('a', 64))] as $bad) {
            try {
                Digest::parse($bad);
                $this->fail("accepted {$bad} as a digest");
            } catch (InvalidArgumentException) {
                $this->addToAssertionCount(1);
            }
        }
    }

    public function test_a_computed_address_parses_back(): void
    {
        $digest = Digest::of('some bytes');
        $this->assertTrue(Digest::parse($digest->value)->equals($digest));
    }
}
