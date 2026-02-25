use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

use crate::types::{BuyerCard, BuyerInstance, Card, CardInstance};

// Thread-local registries for serde: map instance_id -> Card/BuyerCard
thread_local! {
    static CARD_REGISTRY: RefCell<[Card; 128]> = RefCell::new([Card::BasicRed; 128]);
    static BUYER_REGISTRY: RefCell<[BuyerCard; 128]> = RefCell::new([BuyerCard::Textiles2Vermilion; 128]);
}

pub fn set_card_registry(lookup: &[Card; 128]) {
    CARD_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_card_registry() -> [Card; 128] {
    CARD_REGISTRY.with(|r| *r.borrow())
}

pub fn set_buyer_registry(lookup: &[BuyerCard; 128]) {
    BUYER_REGISTRY.with(|r| {
        *r.borrow_mut() = *lookup;
    });
}

pub fn get_buyer_registry() -> [BuyerCard; 128] {
    BUYER_REGISTRY.with(|r| *r.borrow())
}

const BINOM: [[u64; 10]; 129] = {
    let mut table = [[0u64; 10]; 129];
    let mut n = 0usize;
    while n <= 128 {
        table[n][0] = 1;
        let mut k = 1usize;
        while k <= 9 && k <= n {
            table[n][k] = table[n - 1][k - 1] + table[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    table
};

macro_rules! impl_bitset {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub u128);

        impl $name {
            #[inline]
            pub fn new() -> Self {
                $name(0)
            }

            #[inline]
            pub fn insert(&mut self, id: u8) {
                self.0 |= 1u128 << id;
            }

            #[inline]
            pub fn remove(&mut self, id: u8) {
                self.0 &= !(1u128 << id);
            }

            #[inline]
            pub fn contains(&self, id: u8) -> bool {
                (self.0 >> id) & 1 != 0
            }

            #[inline]
            pub fn len(&self) -> u32 {
                self.0.count_ones()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.0 == 0
            }

            #[inline]
            pub fn union(self, other: Self) -> Self {
                $name(self.0 | other.0)
            }

            #[inline]
            pub fn intersection(self, other: Self) -> Self {
                $name(self.0 & other.0)
            }

            #[inline]
            pub fn difference(self, other: Self) -> Self {
                $name(self.0 & !other.0)
            }

            #[inline]
            pub fn pick_random<R: Rng>(&self, rng: &mut R) -> Option<u8> {
                let count = self.0.count_ones();
                if count == 0 {
                    return None;
                }
                let k = rng.random_range(0..count);
                let mut val = self.0;
                for _ in 0..k {
                    val &= val - 1;
                }
                Some(val.trailing_zeros() as u8)
            }

            #[inline]
            pub fn draw<R: Rng>(&mut self, rng: &mut R) -> Option<u8> {
                let count = self.0.count_ones();
                if count == 0 {
                    return None;
                }
                let k = rng.random_range(0..count);
                let mut val = self.0;
                for _ in 0..k {
                    val &= val - 1; // clear lowest set bit
                }
                let pos = val.trailing_zeros() as u8;
                self.0 &= !(1u128 << pos);
                Some(pos)
            }

            #[inline]
            pub fn draw_multiple<R: Rng>(&mut self, count: u32, rng: &mut R) -> Self {
                let n = self.0.count_ones();
                if count == 0 {
                    return $name(0);
                }
                if count >= n {
                    let all = self.0;
                    self.0 = 0;
                    return $name(all);
                }
                let c = count as usize;
                let total = BINOM[n as usize][c];
                let mut k = rng.random_range(0..total);
                let mut remaining = n as usize;
                let mut to_pick = c;
                let mut selected = 0u128;
                let mut bits = self.0;
                while to_pick > 0 {
                    let pos = bits.trailing_zeros();
                    bits &= bits - 1; // clear lowest set bit
                    remaining -= 1;
                    let threshold = BINOM[remaining][to_pick - 1];
                    if k < threshold {
                        selected |= 1u128 << pos;
                        to_pick -= 1;
                    } else {
                        k -= threshold;
                    }
                }
                self.0 &= !selected;
                $name(selected)
            }

            #[inline]
            pub fn iter(self) -> BitIter {
                BitIter(self.0)
            }
        }
    };
}

pub struct BitIter(u128);

impl Iterator for BitIter {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            let pos = self.0.trailing_zeros() as u8;
            self.0 &= self.0 - 1; // clear lowest set bit
            Some(pos)
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let c = self.0.count_ones() as usize;
        (c, Some(c))
    }
}

impl ExactSizeIterator for BitIter {}

impl_bitset!(UnorderedCards);
impl_bitset!(UnorderedBuyers);

// Serde for UnorderedCards: serialize as Vec<CardInstance>, deserialize and rebuild bitset
impl Serialize for UnorderedCards {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let cards: Vec<CardInstance> = CARD_REGISTRY.with(|r| {
            let reg = r.borrow();
            self.iter()
                .map(|id| CardInstance {
                    instance_id: id as u32,
                    card: reg[id as usize],
                })
                .collect()
        });
        cards.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UnorderedCards {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let cards = Vec::<CardInstance>::deserialize(deserializer)?;
        let mut bitset = UnorderedCards::new();
        CARD_REGISTRY.with(|r| {
            let mut reg = r.borrow_mut();
            for c in &cards {
                let id = c.instance_id as u8;
                reg[id as usize] = c.card;
                bitset.insert(id);
            }
        });
        Ok(bitset)
    }
}

// Serde for UnorderedBuyers: serialize as Vec<BuyerInstance>, deserialize and rebuild bitset
impl Serialize for UnorderedBuyers {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let buyers: Vec<BuyerInstance> = BUYER_REGISTRY.with(|r| {
            let reg = r.borrow();
            self.iter()
                .map(|id| BuyerInstance {
                    instance_id: id as u32,
                    buyer: reg[id as usize],
                })
                .collect()
        });
        buyers.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for UnorderedBuyers {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let buyers = Vec::<BuyerInstance>::deserialize(deserializer)?;
        let mut bitset = UnorderedBuyers::new();
        BUYER_REGISTRY.with(|r| {
            let mut reg = r.borrow_mut();
            for b in &buyers {
                let id = b.instance_id as u8;
                reg[id as usize] = b.buyer;
                bitset.insert(id);
            }
        });
        Ok(bitset)
    }
}
