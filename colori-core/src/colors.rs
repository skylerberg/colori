use crate::types::{Color, ColorWheel, NUM_COLORS};

fn is_primary(idx: usize) -> bool {
    idx == 0 || idx == 4 || idx == 8
}

fn is_secondary(idx: usize) -> bool {
    idx == 2 || idx == 6 || idx == 10
}

pub fn can_mix(a: Color, b: Color) -> bool {
    let a_index = a.index();
    let b_index = b.index();
    let diff = if a_index > b_index { a_index - b_index } else { b_index - a_index };
    let distance = diff.min(NUM_COLORS - diff);

    if is_primary(a_index) && is_primary(b_index) {
        return a_index != b_index;
    }

    let one_is_primary = is_primary(a_index) || is_primary(b_index);
    let one_is_secondary = is_secondary(a_index) || is_secondary(b_index);
    one_is_primary && one_is_secondary && distance == 2
}

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
    Color::Indigo,
    Color::Magenta,
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

#[inline]
pub fn perform_mix(wheel: &mut ColorWheel, a: Color, b: Color) -> bool {
    if !can_mix(a, b) {
        return false;
    }
    if wheel.get(a) == 0 || wheel.get(b) == 0 {
        return false;
    }
    wheel.decrement(a);
    wheel.decrement(b);
    let result = mix_result(a, b);
    wheel.increment(result);
    true
}

/// Performs a mix without checking `can_mix` or wheel amounts.
/// The caller must guarantee that `a` and `b` are a valid mix pair
/// and that the wheel has at least one of each.
#[inline]
pub fn perform_mix_unchecked(wheel: &mut ColorWheel, a: Color, b: Color) {
    wheel.decrement(a);
    wheel.decrement(b);
    let result = mix_result(a, b);
    wheel.increment(result);
}

#[inline]
pub fn can_pay_cost(wheel: &ColorWheel, cost: &[Color]) -> bool {
    let mut used = [0u32; 12];
    for &c in cost {
        let idx = c.index();
        let needed = used[idx] + 1;
        if wheel.get(c) < needed {
            return false;
        }
        used[idx] = needed;
    }
    true
}

#[inline]
pub fn pay_cost(wheel: &mut ColorWheel, cost: &[Color]) -> bool {
    if !can_pay_cost(wheel, cost) {
        return false;
    }
    for &c in cost {
        wheel.decrement(c);
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
}
