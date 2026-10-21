use crate::action_phase::{can_afford_sell_card, for_each_unique_card_type};
use crate::types::*;

use super::mix_sequences::enumerate_mix_sequences;
use super::multiset::{count_card_types, enumerate_multiset_subsets, enumerate_multiset_subsets_exact};
use super::should_force_max_workshop;

/// What enumerating a copied card's Workshop ability needs, none of which
/// depends on *which* card was copied: the workshop's card types, and whether
/// holding picks back can pay off.
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
    force_max: bool,
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
                force_max: should_force_max_workshop(state, player),
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
    match card.trash_abilities()[0] {
        Ability::MixColors { count } => {
            enumerate_mix_sequences(
                &player.color_wheel,
                player.wild_primaries,
                count,
                choices,
                |mixes| Choice::TrashAndMix { card, mixes },
            );
        }
        Ability::Sell => {
            let mut has_sell_card = false;
            for sell_card in player.buyers.iter() {
                if can_afford_sell_card(player, &sell_card.sell_card) {
                    has_sell_card = true;
                    choices.push(Choice::TrashAndSell {
                        card,
                        sell_card: sell_card.sell_card,
                    });
                }
            }

            if !has_sell_card {
                choices.push(Choice::TrashDraftedCard { card });
            }
        }
        Ability::Workshop { count } => {
            let tally = tally.get(state, player);
            let (card_types, type_counts, len) =
                (&tally.card_types[..], &tally.type_counts[..], tally.len);
            let total_available: usize = type_counts[..len].iter().map(|&c| c as usize).sum();
            if tally.force_max {
                if total_available == 0 {
                    choices.push(Choice::TrashAndWorkshop {
                        card,
                        workshop_cards: WorkshopCards::new(),
                    });
                } else if total_available <= count as usize {
                    let mut all_cards = WorkshopCards::new();
                    for i in 0..len {
                        for _ in 0..type_counts[i] {
                            all_cards.push(card_types[i]);
                        }
                    }
                    choices.push(Choice::TrashAndWorkshop {
                        card,
                        workshop_cards: all_cards,
                    });
                } else {
                    enumerate_multiset_subsets_exact(
                        &card_types[..len],
                        &type_counts[..len],
                        count as usize,
                        &mut WorkshopCards::new(),
                        choices,
                        &|workshop_cards| Choice::TrashAndWorkshop {
                            card,
                            workshop_cards,
                        },
                    );
                }
            } else {
                choices.push(Choice::TrashAndWorkshop {
                    card,
                    workshop_cards: WorkshopCards::new(),
                });
                if len > 0 {
                    enumerate_multiset_subsets(
                        &card_types[..len],
                        &type_counts[..len],
                        count as usize,
                        &mut WorkshopCards::new(),
                        choices,
                        &|workshop_cards| Choice::TrashAndWorkshop {
                            card,
                            workshop_cards,
                        },
                    );
                }
            }
        }
        Ability::MoveToDraftPool => {
            if player.workshop_cards.is_empty() {
                choices.push(Choice::TrashAndMoveToDraftPool {
                    card,
                    target: None,
                });
            } else {
                for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |target_card| {
                    choices.push(Choice::TrashAndMoveToDraftPool {
                        card,
                        target: Some(target_card),
                    });
                });
            }
        }
        _ => {
            choices.push(Choice::TrashDraftedCard { card });
        }
    }
}
