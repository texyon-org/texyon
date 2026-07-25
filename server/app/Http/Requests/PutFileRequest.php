<?php

declare(strict_types=1);

namespace App\Http\Requests;

use Illuminate\Foundation\Http\FormRequest;
use Illuminate\Support\Facades\Config;

/**
 * Writing a file's complete contents.
 *
 * Text arrives as a string and anything else as an upload, because putting a
 * PNG through JSON costs a third of its size in base64 for no benefit.
 */
final class PutFileRequest extends FormRequest
{
    /** @return array<string, list<string>> */
    public function rules(): array
    {
        return [
            'text' => ['required_without:file', 'string'],
            'file' => ['required_without:text', 'file', 'max:'.$this->maxKilobytes()],
        ];
    }

    /** The bytes to store, however they were sent. */
    public function contents(): string
    {
        $upload = $this->file('file');
        if (is_array($upload)) {
            // `file` is declared as a single upload; an array means the client
            // sent something the rules should already have refused.
            return '';
        }

        if ($upload !== null) {
            return (string) file_get_contents($upload->getRealPath());
        }

        return $this->string('text')->value();
    }

    private function maxKilobytes(): int
    {
        return intdiv(Config::integer('blobs.max_file_bytes'), 1024);
    }
}
