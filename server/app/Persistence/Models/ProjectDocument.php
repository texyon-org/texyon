<?php

declare(strict_types=1);

namespace App\Persistence\Models;

use Illuminate\Database\Eloquent\Model;

/**
 * @property int $project_id
 * @property string $state
 * @property string $epoch
 * @property \Illuminate\Support\Carbon $created_at
 * @property \Illuminate\Support\Carbon $updated_at
 */
final class ProjectDocument extends Model
{
    protected $primaryKey = 'project_id';

    public $incrementing = false;

    /** @var list<string> */
    protected $fillable = ['project_id', 'state', 'epoch'];
}
