<?php

declare(strict_types=1);

namespace Database\Factories;

use App\Models\User;
use App\Persistence\Models\Project;
use Illuminate\Database\Eloquent\Factories\Factory;

/**
 * @extends Factory<Project>
 */
final class ProjectFactory extends Factory
{
    protected $model = Project::class;

    /**
     * Define the model's default state.
     *
     * The return type is inherited from the parent, which states it in terms of
     * the model's own columns.
     */
    public function definition(): array
    {
        return [
            'owner_id' => User::factory(),
            'name' => fake()->sentence(3),
            'primary_path' => 'main.tex',
        ];
    }
}
