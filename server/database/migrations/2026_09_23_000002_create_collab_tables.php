<?php

declare(strict_types=1);

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        // The Yjs document a project is edited through. project_files is kept in step with it.
        Schema::create('project_documents', function (Blueprint $table): void {
            $table->foreignId('project_id')->primary()->constrained()->cascadeOnDelete();
            // Base64, so it reads back the same on SQLite and Postgres.
            $table->longText('state');
            $table->string('epoch', 64);
            $table->timestamps();
        });

        // Blobs uploaded into a project. A document may only name bytes its own project uploaded.
        Schema::create('project_blobs', function (Blueprint $table): void {
            $table->foreignId('project_id')->constrained()->cascadeOnDelete();
            $table->char('digest', 64);
            $table->timestamp('created_at');

            $table->primary(['project_id', 'digest']);
        });

        Schema::dropIfExists('document_operations');
    }

    public function down(): void
    {
        Schema::dropIfExists('project_blobs');
        Schema::dropIfExists('project_documents');
    }
};
