<?php

declare(strict_types=1);

namespace App\Http\Requests;

use App\Domain\Project\Role;
use Illuminate\Foundation\Http\FormRequest;

/** A role a project can be shared with: editor or viewer, never owner. */
final class RoleRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'role' => ['required', 'string', 'in:editor,viewer'],
        ];
    }

    public function role(): Role
    {
        return Role::from($this->string('role')->value());
    }
}
