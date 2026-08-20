<?php

declare(strict_types=1);

namespace App\Http\Requests;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Config;

/** JSON text only. Binary files go through putBytes(), which reads the raw body. */
final class PutFileRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        // present and nullable rather than required, an empty file is a valid
        // file and middleware turns "" into null
        return [
            'text' => ['present', 'nullable', 'string', 'max:'.Config::integer('blobs.max_file_bytes')],
        ];
    }

    public function contents(): string
    {
        return $this->string('text')->value();
    }
}
