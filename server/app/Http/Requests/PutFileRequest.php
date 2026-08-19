<?php

declare(strict_types=1);

namespace App\Http\Requests;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Config;

/** JSON text only. PHP does not parse multipart PUT bodies, so binary uploads would need their own endpoint. */
final class PutFileRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'text' => ['required', 'string', 'max:'.Config::integer('blobs.max_file_bytes')],
        ];
    }

    public function contents(): string
    {
        return $this->string('text')->value();
    }
}
