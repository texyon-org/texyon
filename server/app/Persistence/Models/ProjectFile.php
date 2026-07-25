<?php

declare(strict_types=1);

namespace App\Persistence\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

/**
 * @property int $id
 * @property int $project_id
 * @property string $path
 * @property string $digest
 * @property int $size
 * @property \Illuminate\Support\Carbon $created_at
 * @property \Illuminate\Support\Carbon $updated_at
 */
final class ProjectFile extends Model
{
    /** @var list<string> */
    protected $fillable = ['project_id', 'path', 'digest', 'size'];

    /** @return BelongsTo<Project, $this> */
    public function project(): BelongsTo
    {
        return $this->belongsTo(Project::class);
    }
}
