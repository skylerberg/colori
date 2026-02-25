use crate::types::{Color, NUM_COLORS};

fn is_primary(idx: usize) -> bool {
    PRIMARIES_MASK & (1 << idx) != 0
}

fn is_secondary(idx: usize) -> bool {
    SECONDARIES_MASK & (1 << idx) != 0
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

pub const PRIMARIES_MASK: u16 = (1 << 0) | (1 << 4) | (1 << 8);
pub const SECONDARIES_MASK: u16 = (1 << 2) | (1 << 6) | (1 << 10);
pub const TERTIARIES_MASK: u16 = (1 << 1) | (1 << 3) | (1 << 5) | (1 << 7) | (1 << 9) | (1 << 11);

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
}
