use crate::types::{Color, ColorWheel, NUM_COLORS, SellCardStats};

/// Indices of primary colors on the 12-color wheel: Red(0), Yellow(4), Blue(8).
const PRIMARY_INDICES: [usize; 3] = [0, 4, 8];
/// Indices of secondary colors on the 12-color wheel: Orange(2), Green(6), Purple(10).
const SECONDARY_INDICES: [usize; 3] = [2, 6, 10];

fn is_primary_idx(idx: usize) -> bool {
    PRIMARY_INDICES.contains(&idx)
}

fn is_secondary_idx(idx: usize) -> bool {
    SECONDARY_INDICES.contains(&idx)
}

pub fn can_mix(a: Color, b: Color) -> bool {
    let a_index = a.index();
    let b_index = b.index();
    let diff = if a_index > b_index { a_index - b_index } else { b_index - a_index };
    let distance = diff.min(NUM_COLORS - diff);

    if is_primary_idx(a_index) && is_primary_idx(b_index) {
        return a_index != b_index;
    }

    let one_is_primary = is_primary_idx(a_index) || is_primary_idx(b_index);
    let one_is_secondary = is_secondary_idx(a_index) || is_secondary_idx(b_index);
    one_is_primary && one_is_secondary && distance == 2
}

/// Compute the result of mixing two colors on the 12-color wheel.
/// Colors are arranged: Red(0), Vermilion(1), Orange(2), ..., Fuchsia(11).
/// Mixing two colors produces the color at the midpoint between them
/// on the shorter arc of the wheel.
pub fn mix_result(a: Color, b: Color) -> Color {
    let a_index = a.index();
    let b_index = b.index();
    let n = NUM_COLORS;
    let forward_dist = (b_index + n - a_index) % n;
    if forward_dist <= n / 2 {
        Color::from_index((a_index + forward_dist / 2) % n)
    } else {
        let back_dist = n - forward_dist;
        Color::from_index((a_index + n - back_dist / 2) % n)
    }
}

pub const PRIMARIES: [Color; 3] = [Color::Red, Color::Yellow, Color::Blue];
pub const SECONDARIES: [Color; 3] = [Color::Orange, Color::Green, Color::Purple];
pub const TERTIARIES: [Color; 6] = [
    Color::Vermilion,
    Color::Amber,
    Color::Chartreuse,
    Color::Teal,
    Color::Ultramarine,
    Color::Fuchsia,
];

/// The 9 color pairs that can legally be mixed:
/// 3 primary+primary and 6 primary+adjacent_secondary.
pub const VALID_MIX_PAIRS: [(Color, Color); 9] = [
    // Primary + Primary
    (Color::Red, Color::Yellow),
    (Color::Red, Color::Blue),
    (Color::Yellow, Color::Blue),
    // Primary + Adjacent Secondary
    (Color::Red, Color::Orange),
    (Color::Yellow, Color::Orange),
    (Color::Yellow, Color::Green),
    (Color::Blue, Color::Green),
    (Color::Blue, Color::Purple),
    (Color::Red, Color::Purple),
];

pub fn is_primary(color: Color) -> bool {
    matches!(color, Color::Red | Color::Yellow | Color::Blue)
}

pub fn is_tertiary(color: Color) -> bool {
    TERTIARIES.contains(&color)
}

pub fn unmix_components(color: Color) -> (Color, Color) {
    match color {
        Color::Orange => (Color::Red, Color::Yellow),
        Color::Green => (Color::Yellow, Color::Blue),
        Color::Purple => (Color::Red, Color::Blue),
        Color::Vermilion => (Color::Red, Color::Orange),
        Color::Amber => (Color::Yellow, Color::Orange),
        Color::Chartreuse => (Color::Yellow, Color::Green),
        Color::Teal => (Color::Blue, Color::Green),
        Color::Ultramarine => (Color::Blue, Color::Purple),
        Color::Fuchsia => (Color::Red, Color::Purple),
        _ => panic!("Cannot unmix primary color {:?}", color),
    }
}

pub fn perform_unmix(wheel: &mut ColorWheel, color: Color) -> bool {
    if is_primary(color) || wheel.get(color) == 0 {
        return false;
    }
    let (a, b) = unmix_components(color);
    wheel.decrement(color);
    wheel.increment(a);
    wheel.increment(b);
    true
}

/// Wild primaries needed for `color` to take part in a mix or a payment: none
/// if the wheel has it, one if it is a primary the wheel lacks, and `None` if it
/// is a secondary or tertiary the wheel lacks.
#[inline]
fn wild_primaries_to_supply(wheel: &ColorWheel, color: Color) -> Option<u32> {
    if wheel.get(color) > 0 {
        Some(0)
    } else if is_primary(color) {
        Some(1)
    } else {
        None
    }
}

/// Whether the valid mix pair `(a, b)` can be made from the wheel, with wild
/// primaries covering any primary it lacks.
#[inline]
pub fn can_supply_mix(wheel: &ColorWheel, wild_primaries: u32, a: Color, b: Color) -> bool {
    match (wild_primaries_to_supply(wheel, a), wild_primaries_to_supply(wheel, b)) {
        (Some(x), Some(y)) => x + y <= wild_primaries,
        _ => false,
    }
}

#[inline]
pub fn perform_mix(wheel: &mut ColorWheel, wild_primaries: &mut u32, a: Color, b: Color) -> bool {
    if !can_mix(a, b) || !can_supply_mix(wheel, *wild_primaries, a, b) {
        return false;
    }
    perform_mix_unchecked(wheel, wild_primaries, a, b);
    true
}

/// Performs a mix without checking `can_mix` or `can_supply_mix`; the caller
/// must guarantee both. Stored colours are spent before wild primaries.
#[inline]
pub fn perform_mix_unchecked(wheel: &mut ColorWheel, wild_primaries: &mut u32, a: Color, b: Color) {
    for color in [a, b] {
        if !wheel.decrement(color) {
            *wild_primaries -= 1;
        }
    }
    wheel.increment(mix_result(a, b));
}

/// Wild primaries needed to cover the primaries in `cost` that the wheel lacks, or
/// `None` when a secondary or tertiary is missing.
#[inline]
pub fn wild_primaries_to_cover_cost(wheel: &ColorWheel, cost: &[Color]) -> Option<u32> {
    let mut used = [0u32; 12];
    let mut wild_primaries = 0;
    for &c in cost {
        let idx = c.index();
        if wheel.counts[idx] > used[idx] {
            used[idx] += 1;
        } else if is_primary(c) {
            wild_primaries += 1;
        } else {
            return None;
        }
    }
    Some(wild_primaries)
}

/// [`wild_primaries_to_cover_cost`] for a sell card, from its per-color counts.
#[inline]
pub fn wild_primaries_to_cover_sale(wheel: &ColorWheel, stats: &SellCardStats) -> Option<u32> {
    let mut wild_primaries = 0;
    for &(color, needed) in stats.colors() {
        let short = (needed as u32).saturating_sub(wheel.get(color));
        if short > 0 {
            if !is_primary(color) {
                return None;
            }
            wild_primaries += short;
        }
    }
    Some(wild_primaries)
}

#[inline]
pub fn can_pay_cost(wheel: &ColorWheel, wild_primaries: u32, cost: &[Color]) -> bool {
    wild_primaries_to_cover_cost(wheel, cost).is_some_and(|needed| needed <= wild_primaries)
}

/// Stored colours are spent before wild primaries.
#[inline]
pub fn pay_cost(wheel: &mut ColorWheel, wild_primaries: &mut u32, cost: &[Color]) -> bool {
    if !can_pay_cost(wheel, *wild_primaries, cost) {
        return false;
    }
    for &c in cost {
        if !wheel.decrement(c) {
            *wild_primaries -= 1;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_mix() {
        assert!(can_mix(Color::Red, Color::Yellow));
        assert!(can_mix(Color::Red, Color::Blue));
        assert!(can_mix(Color::Yellow, Color::Blue));
        assert!(!can_mix(Color::Red, Color::Red));
    }

    #[test]
    fn test_primary_mix_results() {
        assert_eq!(mix_result(Color::Red, Color::Yellow), Color::Orange);
        assert_eq!(mix_result(Color::Yellow, Color::Blue), Color::Green);
        assert_eq!(mix_result(Color::Red, Color::Blue), Color::Purple);
    }

    #[test]
    fn test_secondary_tertiary_mix() {
        // Red + Orange = Vermilion
        assert!(can_mix(Color::Red, Color::Orange));
        assert_eq!(mix_result(Color::Red, Color::Orange), Color::Vermilion);

        // Yellow + Orange = Amber
        assert!(can_mix(Color::Yellow, Color::Orange));
        assert_eq!(mix_result(Color::Yellow, Color::Orange), Color::Amber);
    }

    #[test]
    fn test_cannot_mix_secondaries() {
        assert!(!can_mix(Color::Orange, Color::Green));
        assert!(!can_mix(Color::Orange, Color::Purple));
        assert!(!can_mix(Color::Green, Color::Purple));
    }

    #[test]
    fn test_cannot_mix_non_adjacent() {
        assert!(!can_mix(Color::Red, Color::Green));
        assert!(!can_mix(Color::Yellow, Color::Purple));
    }

    #[test]
    fn test_valid_mix_pairs_is_exhaustive() {
        use crate::types::ALL_COLORS;

        // Every pair in VALID_MIX_PAIRS must pass can_mix
        for &(a, b) in &VALID_MIX_PAIRS {
            assert!(can_mix(a, b), "{:?} and {:?} should be mixable", a, b);
        }

        // Every mixable pair must appear in VALID_MIX_PAIRS
        for i in 0..ALL_COLORS.len() {
            for j in (i + 1)..ALL_COLORS.len() {
                let a = ALL_COLORS[i];
                let b = ALL_COLORS[j];
                if can_mix(a, b) {
                    let found = VALID_MIX_PAIRS.iter().any(|&(x, y)| {
                        (x == a && y == b) || (x == b && y == a)
                    });
                    assert!(found, "Mixable pair ({:?}, {:?}) missing from VALID_MIX_PAIRS", a, b);
                }
            }
        }
    }

    #[test]
    fn test_a_wild_primary_covers_a_missing_primary_in_a_mix() {
        let mut wheel = ColorWheel::new();
        wheel.set(Color::Orange, 1);
        let mut wild_primaries = 1;
        assert!(can_supply_mix(&wheel, wild_primaries, Color::Red, Color::Orange));
        assert!(perform_mix(&mut wheel, &mut wild_primaries, Color::Red, Color::Orange));
        assert_eq!(wild_primaries, 0);
        assert_eq!(wheel.get(Color::Vermilion), 1);
        assert_eq!(wheel.get(Color::Orange), 0);
    }

    #[test]
    fn test_a_wild_primary_cannot_cover_a_missing_secondary() {
        let mut wheel = ColorWheel::new();
        wheel.set(Color::Red, 1);
        assert!(!can_supply_mix(&wheel, 5, Color::Red, Color::Orange));
        assert!(!can_pay_cost(&wheel, 5, &[Color::Vermilion, Color::Red]));
    }

    #[test]
    fn test_a_mix_spends_stored_colors_before_wild_primaries() {
        let mut wheel = ColorWheel::new();
        wheel.set(Color::Red, 1);
        let mut wild_primaries = 2;
        assert!(perform_mix(&mut wheel, &mut wild_primaries, Color::Red, Color::Yellow));
        assert_eq!(wild_primaries, 1);
        assert_eq!(wheel.get(Color::Red), 0);
        assert_eq!(wheel.get(Color::Orange), 1);
    }

    #[test]
    fn test_paying_a_cost_spends_wild_primaries_only_for_the_shortfall() {
        let mut wheel = ColorWheel::new();
        wheel.set(Color::Blue, 2);
        let cost = [Color::Blue, Color::Blue, Color::Blue];
        assert!(!can_pay_cost(&wheel, 0, &cost));
        let mut wild_primaries = 2;
        assert!(pay_cost(&mut wheel, &mut wild_primaries, &cost));
        assert_eq!(wild_primaries, 1);
        assert_eq!(wheel.get(Color::Blue), 0);
    }
}
