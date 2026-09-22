<?php

declare(strict_types=1);

namespace App\Persistence\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\BelongsTo;

/**
 * @property int $id
 * @property int $project_id
 * @property string $token_hash
 * @property string $role
 * @property int|null $created_by
 * @property-read Project $project
 * @property \Illuminate\Support\Carbon $created_at
 * @property \Illuminate\Support\Carbon $updated_at
 */
final class ProjectInvite extends Model
{
    /** @var list<string> */
    protected $fillable = ['project_id', 'token_hash', 'role', 'created_by'];

    /** @return BelongsTo<Project, $this> */
    public function project(): BelongsTo
    {
        return $this->belongsTo(Project::class);
    }
}
