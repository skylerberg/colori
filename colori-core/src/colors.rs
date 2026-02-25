use crate::types::{Color, NUM_COLORS};

fn is_primary(idx: usize) -> bool {
    idx == 0 || idx == 4 || idx == 8
}

fn is_secondary(idx: usize) -> bool {
    idx == 2 || idx == 6 || idx == 10
}

pub fn can_mix(a: Color, b: Color) -> bool {
    let ai = a.index();
    let bi = b.index();
    let diff = if ai > bi { ai - bi } else { bi - ai };
    let distance = diff.min(NUM_COLORS - diff);

    if is_primary(ai) && is_primary(bi) {
        return ai != bi;
    }

    let one_is_primary = is_primary(ai) || is_primary(bi);
    let one_is_secondary = is_secondary(ai) || is_secondary(bi);
    one_is_primary && one_is_secondary && distance == 2
}

pub fn mix_result(a: Color, b: Color) -> Color {
    let ai = a.index();
    let bi = b.index();
    let n = NUM_COLORS;
    let forward_dist = (bi + n - ai) % n;
    if forward_dist <= n / 2 {
        Color::from_index((ai + forward_dist / 2) % n)
    } else {
        let back_dist = n - forward_dist;
        Color::from_index((ai + n - back_dist / 2) % n)
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
