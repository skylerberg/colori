use crate::action_phase::{can_afford_buyer, for_each_unique_card_type};
use crate::types::*;

use super::mix_sequences::enumerate_mix_sequences;
use super::multiset::{count_card_types, enumerate_multiset_subsets};
use super::WorkshopPruning;

/// What enumerating a copied card's Workshop ability needs, none of which
/// depends on *which* card was copied: the workshop's card types, and what
/// decides whether holding picks back can pay off.
///
/// A hand often holds several Workshop cards, and this was tallied once per
/// copy. It is filled in on first use, so a hand with no Workshop card pays
/// nothing for it.
#[derive(Default)]
pub(super) struct WorkshopTally {
    inner: Option<WorkshopTallyInner>,
}

struct WorkshopTallyInner {
    card_types: [Card; NUM_CARDS],
    type_counts: [u8; NUM_CARDS],
    len: usize,
    pruning: WorkshopPruning,
}

impl WorkshopTally {
    fn get(&mut self, state: &GameState, player: &PlayerState) -> &WorkshopTallyInner {
        self.inner.get_or_insert_with(|| {
            let (card_types, type_counts, len) =
                count_card_types(player.workshop_cards, &state.card_lookup);
            WorkshopTallyInner {
                card_types,
                type_counts,
                len,
                pruning: WorkshopPruning::new(state, player),
            }
        })
    }
}

pub(super) fn enumerate_trash_choices(
    state: &GameState,
    player: &PlayerState,
    card: Card,
    tally: &mut WorkshopTally,
    choices: &mut Vec<Choice>,
) {
    // What follows the first ability is left on the stack and chosen for on
    // its own once this choice has resolved.
    match card.top_actions()[0] {
        Ability::MixColors { count } => {
            enumerate_mix_sequences(
                &player.color_wheel,
                player.bowls,
                count,
                choices,
                |mixes| Choice::TrashAndMix { card, mixes },
            );
        }
        Ability::Sell => {
            let mut has_buyer = false;
            for buyer in player.buyers.iter() {
                if can_afford_buyer(player, &buyer.buyer) {
                    has_buyer = true;
                    choices.push(Choice::TrashAndSell {
                        card,
                        buyer: buyer.buyer,
                    });
                }
            }

            if !has_buyer {
                choices.push(Choice::TrashTopCard { card });
            }
        }
        Ability::Workshop { count } => {
            let tally = tally.get(state, player);
            let (fewest, most) = tally.pruning.pick_range(count);
            let (card_types, type_counts, len) =
                (&tally.card_types[..], &tally.type_counts[..], tally.len);
            if fewest == 0 {
                // Every card left could still be moved, so forfeiting the picks
                // is a real option: the card is trashed either way, and only the
                // picks are given up.
                choices.push(Choice::TrashAndWorkshop {
                    card,
                    workshop_cards: WorkshopCards::new(),
                });
            }
            if most > 0 {
                enumerate_multiset_subsets(
                    &card_types[..len],
                    &type_counts[..len],
                    fewest.max(1),
                    most,
                    &mut WorkshopCards::new(),
                    choices,
                    &|workshop_cards| Choice::TrashAndWorkshop {
                        card,
                        workshop_cards,
                    },
                );
            }
        }
        Ability::LiftCard => {
            if player.workshop_cards.is_empty() {
                choices.push(Choice::TrashAndLift {
                    card,
                    target: None,
                });
            } else {
                for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |target_card| {
                    choices.push(Choice::TrashAndLift {
                        card,
                        target: Some(target_card),
                    });
                });
            }
        }
        _ => {
            choices.push(Choice::TrashTopCard { card });
        }
    }
}
