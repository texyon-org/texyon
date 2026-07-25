<?php

declare(strict_types=1);

namespace App\Http\Requests;

use App\Domain\Project\ProjectPath;
use Illuminate\Foundation\Http\FormRequest;

/**
 * Creating a project.
 *
 * Validation says the shape is plausible; the accessors below are what turn it
 * into something typed. `validated()` hands back `array<string, mixed>`, and
 * letting that travel any further would put a `mixed` into every method it
 * touched.
 */
final class StoreProjectRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'name' => ['required', 'string', 'max:120'],
            'primary' => ['sometimes', 'string', 'max:512'],
        ];
    }

    public function name(): string
    {
        return $this->string('name')->trim()->value();
    }

    public function primary(): ProjectPath
    {
        $raw = $this->string('primary')->trim()->value();

        return ProjectPath::parse($raw === '' ? 'main.tex' : $raw);
    }
}
