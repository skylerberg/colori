use crate::action_phase::{can_afford_sell_card, for_each_unique_card_type};
use crate::colors::PRIMARIES;
use crate::types::*;
use smallvec::SmallVec;

use super::mix_sequences::enumerate_mix_sequences;
use super::multiset::{count_card_types, enumerate_multiset_subsets};

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
            // Glass card acquisition
            if state.expansions.glass {
                for gi in state.glass_display.iter() {
                    for &color in &PRIMARIES {
                        if player.color_wheel.get(color) >= 4 {
                            has_sell_card = true;
                            choices.push(Choice::DestroyAndSelectGlass {
                                card,
                                glass: gi.card,
                                pay_color: color,
                            });
                        }
                    }
                }
            }
            if !has_sell_card {
                choices.push(Choice::DestroyDraftedCard { card });
            }
        }
        Ability::Workshop { count } => {
            // Skip option (empty workshop_cards)
            choices.push(Choice::DestroyAndWorkshop {
                card,
                workshop_cards: SmallVec::new(),
            });
            let (card_types, type_counts, len) =
                count_card_types(player.workshop_cards, &state.card_lookup);
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
        Ability::DestroyCards => {
            if player.workshop_cards.is_empty() {
                choices.push(Choice::DestroyAndDestroyCards {
                    card,
                    target: None,
                });
            } else {
                for_each_unique_card_type(&player.workshop_cards, &state.card_lookup, |target_card| {
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
