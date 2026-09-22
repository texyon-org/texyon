<?php

declare(strict_types=1);

namespace App\Http\Requests;

use App\Domain\Project\Role;
use Illuminate\Foundation\Http\FormRequest;

/** Share a project with an existing account. */
final class ShareProjectRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'email' => ['required', 'string', 'email', 'max:255'],
            'role' => ['required', 'string', 'in:editor,viewer'],
        ];
    }

    public function email(): string
    {
        return $this->string('email')->trim()->lower()->value();
    }

    public function role(): Role
    {
        return Role::from($this->string('role')->value());
    }
}
