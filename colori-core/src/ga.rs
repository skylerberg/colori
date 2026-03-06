use crate::rollout_policy::TOTAL_WEIGHTS;
use rand::Rng;
use rand::RngExt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Individual {
    pub weights: [f64; TOTAL_WEIGHTS],
    pub fitness: f64,
}

pub fn random_individual<R: Rng>(rng: &mut R) -> Individual {
    let mut weights = [0.0f64; TOTAL_WEIGHTS];
    for w in weights.iter_mut() {
        // Box-Muller transform for Gaussian(0, 1)
        let u1: f64 = rng.random_range(1e-10..1.0f64);
        let u2: f64 = rng.random_range(0.0..std::f64::consts::TAU);
        *w = (-2.0 * u1.ln()).sqrt() * u2.cos();
    }
    Individual {
        weights,
        fitness: 0.0,
    }
}

pub fn tournament_select<'a, R: Rng>(
    population: &'a [Individual],
    tournament_size: usize,
    rng: &mut R,
) -> &'a Individual {
    let mut best_idx = rng.random_range(0..population.len());
    for _ in 1..tournament_size {
        let idx = rng.random_range(0..population.len());
        if population[idx].fitness > population[best_idx].fitness {
            best_idx = idx;
        }
    }
    &population[best_idx]
}

pub fn blx_crossover<R: Rng>(
    parent1: &Individual,
    parent2: &Individual,
    alpha: f64,
    rng: &mut R,
) -> Individual {
    let mut weights = [0.0f64; TOTAL_WEIGHTS];
    for i in 0..TOTAL_WEIGHTS {
        let lo = parent1.weights[i].min(parent2.weights[i]);
        let hi = parent1.weights[i].max(parent2.weights[i]);
        let range = hi - lo;
        if range < 1e-12 {
            weights[i] = lo;
        } else {
            let ext = range * alpha;
            weights[i] = rng.random_range((lo - ext)..(hi + ext));
        }
    }
    Individual {
        weights,
        fitness: 0.0,
    }
}

pub fn gaussian_mutate<R: Rng>(
    individual: &mut Individual,
    mutation_rate: f64,
    sigma: f64,
    rng: &mut R,
) {
    for w in individual.weights.iter_mut() {
        if rng.random_range(0.0..1.0f64) < mutation_rate {
            let u1: f64 = rng.random_range(1e-10..1.0f64);
            let u2: f64 = rng.random_range(0.0..std::f64::consts::TAU);
            let noise = sigma * (-2.0 * u1.ln()).sqrt() * u2.cos();
            *w += noise;
        }
    }
}

pub struct GAConfig {
    pub tournament_size: usize,
    pub crossover_alpha: f64,
    pub mutation_rate: f64,
    pub mutation_sigma: f64,
    pub elitism_count: usize,
}

impl Default for GAConfig {
    fn default() -> Self {
        GAConfig {
            tournament_size: 3,
            crossover_alpha: 0.5,
            mutation_rate: 0.3,
            mutation_sigma: 0.5,
            elitism_count: 2,
        }
    }
}

pub fn evolve_generation<R: Rng>(
    population: &[Individual],
    config: &GAConfig,
    rng: &mut R,
) -> Vec<Individual> {
    let pop_size = population.len();
    let mut next_gen = Vec::with_capacity(pop_size);

    // Elitism: copy the best individuals directly
    let mut sorted_indices: Vec<usize> = (0..pop_size).collect();
    sorted_indices.sort_by(|&a, &b| {
        population[b]
            .fitness
            .partial_cmp(&population[a].fitness)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for i in 0..config.elitism_count.min(pop_size) {
        next_gen.push(population[sorted_indices[i]].clone());
    }

    // Fill rest with crossover + mutation
    while next_gen.len() < pop_size {
        let p1 = tournament_select(population, config.tournament_size, rng);
        let p2 = tournament_select(population, config.tournament_size, rng);
        let mut child = blx_crossover(p1, p2, config.crossover_alpha, rng);
        gaussian_mutate(&mut child, config.mutation_rate, config.mutation_sigma, rng);
        next_gen.push(child);
    }

    next_gen
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use wyrand::WyRand;

    #[test]
    fn test_random_individual() {
        let mut rng = WyRand::seed_from_u64(42);
        let ind = random_individual(&mut rng);
        assert_eq!(ind.weights.len(), TOTAL_WEIGHTS);
        assert_eq!(ind.fitness, 0.0);
        // Weights should not all be zero (extremely unlikely with random init)
        assert!(ind.weights.iter().any(|&w| w != 0.0));
    }

    #[test]
    fn test_tournament_select_picks_best() {
        let mut rng = WyRand::seed_from_u64(42);
        let pop: Vec<Individual> = (0..10)
            .map(|i| Individual {
                weights: [i as f64; TOTAL_WEIGHTS],
                fitness: i as f64,
            })
            .collect();
        // With tournament size = population size, should always pick the best
        let selected = tournament_select(&pop, 10, &mut rng);
        assert_eq!(selected.fitness, 9.0);
    }

    #[test]
    fn test_evolve_generation_preserves_size() {
        let mut rng = WyRand::seed_from_u64(42);
        let pop: Vec<Individual> = (0..20).map(|_| random_individual(&mut rng)).collect();
        let mut pop_with_fitness = pop;
        for (i, ind) in pop_with_fitness.iter_mut().enumerate() {
            ind.fitness = i as f64;
        }
        let config = GAConfig::default();
        let next = evolve_generation(&pop_with_fitness, &config, &mut rng);
        assert_eq!(next.len(), pop_with_fitness.len());
    }

    #[test]
    fn test_blx_crossover_in_range() {
        let mut rng = WyRand::seed_from_u64(42);
        let p1 = Individual {
            weights: [1.0; TOTAL_WEIGHTS],
            fitness: 0.0,
        };
        let p2 = Individual {
            weights: [3.0; TOTAL_WEIGHTS],
            fitness: 0.0,
        };
        let child = blx_crossover(&p1, &p2, 0.5, &mut rng);
        // With alpha=0.5, range extends to [0.0, 4.0]
        for &w in &child.weights {
            assert!(w >= -0.5 && w <= 4.5, "weight {} out of range", w);
        }
    }
}
