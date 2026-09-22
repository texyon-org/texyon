<?php

declare(strict_types=1);

namespace App\Http\Requests;

use App\Domain\Collab\StoredDocument;
use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Config;

/** A document from the collaboration server, with the files it currently holds. */
final class StoreDocumentRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        $max = 'max:'.Config::integer('blobs.max_file_bytes');

        return [
            'state' => ['required', 'string'],
            'epoch' => ['required', 'string', 'max:64'],
            'files' => ['present', 'array'],
            'files.*.path' => ['required', 'string', 'max:512'],
            'files.*.text' => ['present', 'string', $max],
            'binaries' => ['present', 'array'],
            'binaries.*.path' => ['required', 'string', 'max:512'],
            'binaries.*.digest' => ['required', 'string', 'size:64'],
            'binaries.*.size' => ['required', 'integer', 'min:0'],
        ];
    }

    public function document(): StoredDocument
    {
        $state = base64_decode($this->string('state')->value(), true);

        return new StoredDocument($state === false ? '' : $state, $this->string('epoch')->value());
    }

    /** @return list<array{path: string, text: string}> */
    public function files(): array
    {
        $files = [];
        foreach ($this->array('files') as $file) {
            if (is_array($file) && is_string($file['path'] ?? null) && is_string($file['text'] ?? null)) {
                $files[] = ['path' => $file['path'], 'text' => $file['text']];
            }
        }

        return $files;
    }

    /** @return list<array{path: string, digest: string, size: int}> */
    public function binaries(): array
    {
        $binaries = [];
        foreach ($this->array('binaries') as $binary) {
            if (is_array($binary) && is_string($binary['path'] ?? null) && is_string($binary['digest'] ?? null) && is_int($binary['size'] ?? null)) {
                $binaries[] = ['path' => $binary['path'], 'digest' => $binary['digest'], 'size' => $binary['size']];
            }
        }

        return $binaries;
    }
}
