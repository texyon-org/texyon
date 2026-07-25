<?php

declare(strict_types=1);

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('projects', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('owner_id')->constrained('users')->cascadeOnDelete();
            $table->string('name');
            // Which file TeX is pointed at. Stored rather than assumed, since a
            // project is free to call it something other than main.tex.
            $table->string('primary_path', 512);
            $table->timestamps();

            $table->index(['owner_id', 'updated_at']);
        });

        Schema::create('project_files', function (Blueprint $table): void {
            $table->id();
            $table->foreignId('project_id')->constrained()->cascadeOnDelete();
            $table->string('path', 512);
            // The content address of the bytes. Several files, in this project
            // or another, may share one: the blob store holds one copy.
            $table->char('digest', 64);
            $table->unsignedBigInteger('size');
            $table->timestamps();

            // A path names at most one thing in a project, and the database is
            // where that is true rather than wherever the check was remembered.
            $table->unique(['project_id', 'path']);
            $table->index('digest');
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('project_files');
        Schema::dropIfExists('projects');
    }
};
