use crate::colors::{can_supply_mix, perform_mix_unchecked, VALID_MIX_PAIRS};
use crate::types::*;
use smallvec::SmallVec;

type Outcome = ([u8; NUM_COLORS], u32);

/// Offer every distinct result of making up to `remaining_mixes` mixes,
/// starting with making none.
///
/// Sequences that leave the same wheel and bowls are one result, and only the
/// first found is offered: the order of mixes shows nowhere else, and three
/// mixes over nine pairs is too wide to search with every permutation in it.
pub(super) fn enumerate_mix_sequences<F>(
    wheel: &ColorWheel,
    bowls: u32,
    remaining_mixes: u32,
    choices: &mut Vec<Choice>,
    make_choice: F,
) where
    F: Fn(MixSequence) -> Choice,
{
    let mut outcomes: SmallVec<[Outcome; 32]> = SmallVec::new();
    extend_sequence(
        wheel,
        bowls,
        remaining_mixes.min(MAX_MIX_SEQUENCE as u32),
        &mut MixSequence::new(),
        &mut outcomes,
        choices,
        &make_choice,
    );
}

fn extend_sequence<F>(
    wheel: &ColorWheel,
    bowls: u32,
    remaining_mixes: u32,
    sequence: &mut MixSequence,
    outcomes: &mut SmallVec<[Outcome; 32]>,
    choices: &mut Vec<Choice>,
    make_choice: &F,
) where
    F: Fn(MixSequence) -> Choice,
{
    // Every mix shrinks the stock by one, so a repeated outcome was reached in
    // the same number of mixes and has the same budget left: nothing below it
    // is lost by stopping here.
    let outcome = (wheel.counts, bowls);
    if outcomes.contains(&outcome) {
        return;
    }
    outcomes.push(outcome);
    choices.push(make_choice(sequence.clone()));

    if remaining_mixes == 0 {
        return;
    }
    for &(a, b) in &VALID_MIX_PAIRS {
        if can_supply_mix(wheel, bowls, a, b) {
            let mut next_wheel = wheel.clone();
            let mut next_bowls = bowls;
            perform_mix_unchecked(&mut next_wheel, &mut next_bowls, a, b);
            sequence.push((a, b));
            extend_sequence(
                &next_wheel,
                next_bowls,
                remaining_mixes - 1,
                sequence,
                outcomes,
                choices,
                make_choice,
            );
            sequence.pop();
        }
    }
}
