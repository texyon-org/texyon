<?php

namespace App\Actions\Fortify;

use App\Models\User;
use Illuminate\Support\Facades\Hash;
use Illuminate\Support\Facades\Validator;
use Illuminate\Validation\Rule;
use Illuminate\Validation\ValidationException;
use Laravel\Fortify\Contracts\CreatesNewUsers;

class CreateNewUser implements CreatesNewUsers
{
    use PasswordValidationRules;

    /**
     * Validate and create a newly registered user.
     *
     * @param  array<array-key, mixed>  $input
     *
     * @throws ValidationException
     */
    public function create(array $input): User
    {
        Validator::make($input, [
            'name' => ['required', 'string', 'max:255'],
            'email' => [
                'required',
                'string',
                'email',
                'max:255',
                Rule::unique(User::class),
            ],
            'password' => $this->passwordRules(),
        ])->validate();

        // Validation has already run, but to static analysis it proves shape
        // rather than type: the contract's parameter is a bare array, so what
        // comes out of it is mixed until something narrows it. A cast would
        // turn an array into "Array" and call it a name.
        return User::create([
            'name' => self::text($input, 'name'),
            'email' => self::text($input, 'email'),
            'password' => Hash::make(self::text($input, 'password')),
        ]);
    }

    /**
     * One validated field, as the string it was validated to be.
     *
     * @param  array<array-key, mixed>  $input
     */
    private static function text(array $input, string $key): string
    {
        $value = $input[$key] ?? null;

        return is_string($value) ? $value : '';
    }
}
