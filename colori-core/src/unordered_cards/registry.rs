use std::cell::RefCell;

use crate::types::{SellCard, Card, ALL_SELL_CARDS};

// Thread-local registries for serde: map instance_id -> Card/SellCard
thread_local! {
    pub(super) static CARD_REGISTRY: RefCell<[Card; 256]> = RefCell::new([Card::BasicRed; 256]);
    pub(super) static SELL_CARD_REGISTRY: RefCell<[SellCard; 256]> = RefCell::new([ALL_SELL_CARDS[0]; 256]);
}

pub fn set_card_registry(lookup: &[Card; 256]) {
    CARD_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_card_registry() -> [Card; 256] {
    CARD_REGISTRY.with(|r| *r.borrow())
}

pub fn set_sell_card_registry(lookup: &[SellCard; 256]) {
    SELL_CARD_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_sell_card_registry() -> [SellCard; 256] {
    SELL_CARD_REGISTRY.with(|r| *r.borrow())
}
