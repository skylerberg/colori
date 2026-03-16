use crate::colors::{perform_mix_unchecked, VALID_MIX_PAIRS};
use crate::types::*;
use smallvec::SmallVec;

pub(super) fn enumerate_mix_sequences<F>(
    wheel: &ColorWheel,
    remaining_mixes: u32,
    choices: &mut Vec<Choice>,
    make_choice: F,
) where
    F: Fn(SmallVec<[(Color, Color); 2]>) -> Choice,
{
    // Always include skip-all (empty mixes)
    choices.push(make_choice(SmallVec::new()));

    for &(a, b) in &VALID_MIX_PAIRS {
        if wheel.get(a) > 0 && wheel.get(b) > 0 {
            let mut mixes1 = SmallVec::new();
            mixes1.push((a, b));
            choices.push(make_choice(mixes1));

            if remaining_mixes > 1 {
                let mut wheel2 = wheel.clone();
                perform_mix_unchecked(&mut wheel2, a, b);
                for &(c, d) in &VALID_MIX_PAIRS {
                    if wheel2.get(c) > 0 && wheel2.get(d) > 0 {
                        let mut mixes2 = SmallVec::new();
                        mixes2.push((a, b));
                        mixes2.push((c, d));
                        choices.push(make_choice(mixes2));
                    }
                }
            }
        }
    }
}
