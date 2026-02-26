use crate::colors::{can_mix, mix_result};
use crate::types::{Color, ColorWheel};

#[inline]
pub fn store_color(wheel: &mut ColorWheel, color: Color) {
    wheel.increment(color);
}

#[inline]
pub fn remove_color(wheel: &mut ColorWheel, color: Color) -> bool {
    wheel.decrement(color)
}

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
