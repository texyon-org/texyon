<?php

declare(strict_types=1);

return [

    /*
    |--------------------------------------------------------------------------
    | Blob disk
    |--------------------------------------------------------------------------
    |
    | Which of the disks in config/filesystems.php holds project content. The
    | store above it is content-addressed and does not care: `local` keeps
    | blobs on a mounted volume, `s3` puts them in a bucket, and the
    | application cannot tell the difference.
    |
    */

    'disk' => env('BLOB_DISK', 'blobs'),

    /*
    |--------------------------------------------------------------------------
    | Direct download lifetime
    |--------------------------------------------------------------------------
    |
    | How long a signed URL handed to a browser stays valid. Only disks that
    | can sign one use this; a local disk has no such notion and the bytes are
    | served by the application instead.
    |
    */

    'url_lifetime' => (int) env('BLOB_URL_LIFETIME', 300),

    /*
    |--------------------------------------------------------------------------
    | Largest file
    |--------------------------------------------------------------------------
    |
    | An upload above this is refused. Figures are the reason this is measured
    | in megabytes rather than kilobytes.
    |
    */

    'max_file_bytes' => (int) env('BLOB_MAX_FILE_BYTES', 32 * 1024 * 1024),

];
