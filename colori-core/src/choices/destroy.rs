use crate::action_phase::{can_afford_sell_card, for_each_unique_card_type_in_workshop_area};
use crate::types::*;
use smallvec::SmallVec;

use super::mix_sequences::enumerate_mix_sequences;
use super::multiset::{count_card_types, enumerate_multiset_subsets, enumerate_multiset_subsets_exact};
use super::should_force_max_workshop;

pub(super) fn enumerate_destroy_choices(
    state: &GameState,
    player: &PlayerState,
    card: Card,
    choices: &mut Vec<Choice>,
) {
    match card.ability() {
        Ability::MixColors { count } => {
            enumerate_mix_sequences(
                &player.color_wheel,
                count,
                choices,
                |mixes| Choice::DestroyAndMix { card, mixes },
            );
        }
        Ability::Sell => {
            let mut has_sell_card = false;
            for sell_card in state.sell_card_display.iter() {
                if can_afford_sell_card(player, &sell_card.sell_card) {
                    has_sell_card = true;
                    choices.push(Choice::DestroyAndSell {
                        card,
                        sell_card: sell_card.sell_card,
                    });
                }
            }

            if !has_sell_card {
                choices.push(Choice::DestroyDraftedCard { card });
            }
        }
        Ability::Workshop { count } => {
            let force_max = should_force_max_workshop(state, player);
            let (card_types, type_counts, len) =
                count_card_types(player.workshop_cards, &state.card_lookup);
            let total_available: usize = type_counts[..len].iter().map(|&c| c as usize).sum();
            if force_max {
                if total_available == 0 {
                    choices.push(Choice::DestroyAndWorkshop {
                        card,
                        workshop_cards: SmallVec::new(),
                    });
                } else if total_available <= count as usize {
                    let mut all_cards = SmallVec::new();
                    for i in 0..len {
                        for _ in 0..type_counts[i] {
                            all_cards.push(card_types[i]);
                        }
                    }
                    choices.push(Choice::DestroyAndWorkshop {
                        card,
                        workshop_cards: all_cards,
                    });
                } else {
                    enumerate_multiset_subsets_exact(
                        &card_types[..len],
                        &type_counts[..len],
                        count as usize,
                        &mut SmallVec::new(),
                        choices,
                        &|workshop_cards| Choice::DestroyAndWorkshop {
                            card,
                            workshop_cards,
                        },
                    );
                }
            } else {
                choices.push(Choice::DestroyAndWorkshop {
                    card,
                    workshop_cards: SmallVec::new(),
                });
                if len > 0 {
                    enumerate_multiset_subsets(
                        &card_types[..len],
                        &type_counts[..len],
                        count as usize,
                        &mut SmallVec::new(),
                        choices,
                        &|workshop_cards| Choice::DestroyAndWorkshop {
                            card,
                            workshop_cards,
                        },
                    );
                }
            }
        }
        Ability::DestroyCards => {
            if player.workshop_cards.is_empty() && player.workshopped_cards.is_empty() {
                choices.push(Choice::DestroyAndDestroyCards {
                    card,
                    target: None,
                });
            } else {
                for_each_unique_card_type_in_workshop_area(player, &state.card_lookup, |target_card| {
                    choices.push(Choice::DestroyAndDestroyCards {
                        card,
                        target: Some(target_card),
                    });
                });
            }
        }
        _ => {
            choices.push(Choice::DestroyDraftedCard { card });
        }
    }
}
