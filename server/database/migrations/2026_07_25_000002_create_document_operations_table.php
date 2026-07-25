<?php

declare(strict_types=1);

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    /**
     * An append-only log of edits, for collaborative editing later.
     *
     * Nothing writes to this yet, and that is deliberate. The editor ships with
     * one writer per file, but the shape of the thing that replaces it — a CRDT
     * or an operation log — decides whether the file row can stay the single
     * source of truth. Reserving the table now costs one migration; discovering
     * later that project_files was the wrong place for authority costs a
     * rewrite of everything that reads it.
     */
    public function up(): void
    {
        Schema::create('document_operations', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('project_file_id')->constrained()->cascadeOnDelete();
            $table->foreignId('author_id')->nullable()->constrained('users')->nullOnDelete();
            // Opaque here on purpose: what an operation *is* belongs to whatever
            // editing model wins, and this table should not have an opinion.
            $table->binary('payload');
            $table->unsignedBigInteger('sequence');
            $table->timestamp('created_at');

            $table->unique(['project_file_id', 'sequence']);
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('document_operations');
    }
};
