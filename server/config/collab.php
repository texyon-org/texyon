<?php

declare(strict_types=1);

return [

    // Where the collaboration server listens. Laravel only calls it to disconnect people.
    'url' => env('COLLAB_URL', 'http://127.0.0.1:1234'),

    // How long a connection token stays usable. Only the handshake reads it.
    'token_ttl' => (int) env('COLLAB_TOKEN_TTL', 60),

];
