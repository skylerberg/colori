use std::cell::RefCell;

use crate::types::{Buyer, Card, ALL_BUYERS};

// Thread-local registries for serde: map instance_id -> Card/Buyer
thread_local! {
    pub(super) static CARD_REGISTRY: RefCell<[Card; 256]> = RefCell::new([Card::BasicRed; 256]);
    pub(super) static BUYER_REGISTRY: RefCell<[Buyer; 256]> = RefCell::new([ALL_BUYERS[0]; 256]);
}

pub fn set_card_registry(lookup: &[Card; 256]) {
    CARD_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_card_registry() -> [Card; 256] {
    CARD_REGISTRY.with(|r| *r.borrow())
}

pub fn set_buyer_registry(lookup: &[Buyer; 256]) {
    BUYER_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_buyer_registry() -> [Buyer; 256] {
    BUYER_REGISTRY.with(|r| *r.borrow())
}
