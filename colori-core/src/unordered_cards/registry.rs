use std::cell::RefCell;

use crate::types::{BuyerCard, Card};

// Thread-local registries for serde: map instance_id -> Card/BuyerCard
thread_local! {
    pub(super) static CARD_REGISTRY: RefCell<[Card; 256]> = RefCell::new([Card::BasicRed; 256]);
    pub(super) static BUYER_REGISTRY: RefCell<[BuyerCard; 256]> = RefCell::new([BuyerCard::Textiles2Vermilion; 256]);
}

pub fn set_card_registry(lookup: &[Card; 256]) {
    CARD_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_card_registry() -> [Card; 256] {
    CARD_REGISTRY.with(|r| *r.borrow())
}

pub fn set_buyer_registry(lookup: &[BuyerCard; 256]) {
    BUYER_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_buyer_registry() -> [BuyerCard; 256] {
    BUYER_REGISTRY.with(|r| *r.borrow())
}
