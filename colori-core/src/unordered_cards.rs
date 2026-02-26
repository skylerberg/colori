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

const BINOM_CUM: [[u64; 10]; 129] = {
    let mut table = [[0u64; 10]; 129];
    let mut n = 0usize;
    while n <= 128 {
        table[n][0] = BINOM[n][0];
        let mut k = 1usize;
        while k <= 9 {
            table[n][k] = table[n][k - 1] + BINOM[n][k];
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
            pub fn draw_up_to<R: Rng>(&mut self, max_count: u8, rng: &mut R) -> Self {
                if max_count == 1 {
                    let n = self.0.count_ones();
                    if n == 0 { return $name(0); }
                    let r = rng.random_range(0..(n as u64 + 1));
                    if r == 0 { return $name(0); }
                    let mut val = self.0;
                    for _ in 0..(r - 1) {
                        val &= val - 1;
                    }
                    let pos = val.trailing_zeros();
                    self.0 &= !(1u128 << pos);
                    return $name(1u128 << pos);
                }
                let n = self.0.count_ones() as usize;
                let c = (max_count as usize).min(n);
                if c == 0 {
                    return $name(0);
                }
                let total = BINOM_CUM[n][c];
                let mut r = rng.random_range(0..total);
                // Find which size bucket using cumulative table
                let mut size = 0usize;
                for k in 0..=c {
                    if r < BINOM_CUM[n][k] {
                        size = k;
                        if k > 0 {
                            r -= BINOM_CUM[n][k - 1];
                        }
                        break;
                    }
                }
                if size == 0 {
                    return $name(0);
                }
                // Unrank within the size bucket using combinatorial number system
                let mut remaining = n;
                let mut to_pick = size;
                let mut selected = 0u128;
                let mut bits = self.0;
                while to_pick > 0 {
                    let pos = bits.trailing_zeros();
                    bits &= bits - 1;
                    remaining -= 1;
                    let threshold = BINOM[remaining][to_pick - 1];
                    if r < threshold {
                        selected |= 1u128 << pos;
                        to_pick -= 1;
                    } else {
                        r -= threshold;
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

impl Default for UnorderedCards {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UnorderedBuyers {
    fn default() -> Self {
        Self::new()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    // ── Basic operations ──

    #[test]
    fn test_new_is_empty() {
        let s = UnorderedCards::new();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
    }

    #[test]
    fn test_insert_single() {
        let mut s = UnorderedCards::new();
        s.insert(10);
        assert!(s.contains(10));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_insert_multiple() {
        let mut s = UnorderedCards::new();
        s.insert(0);
        s.insert(5);
        s.insert(127);
        assert_eq!(s.len(), 3);
        assert!(s.contains(0));
        assert!(s.contains(5));
        assert!(s.contains(127));
    }

    #[test]
    fn test_insert_idempotent() {
        let mut s = UnorderedCards::new();
        s.insert(42);
        assert_eq!(s.len(), 1);
        s.insert(42);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut s = UnorderedCards::new();
        s.insert(7);
        s.remove(7);
        assert!(s.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut s = UnorderedCards::new();
        s.remove(50);
        assert!(s.is_empty());
    }

    #[test]
    fn test_contains_empty() {
        let s = UnorderedCards::new();
        assert!(!s.contains(0));
        assert!(!s.contains(64));
        assert!(!s.contains(127));
    }

    #[test]
    fn test_high_bits() {
        let mut s = UnorderedCards::new();
        s.insert(127);
        assert!(s.contains(127));
        assert_eq!(s.len(), 1);
    }

    // ── Set operations ──

    #[test]
    fn test_union() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        let mut b = UnorderedCards::new();
        b.insert(1);
        b.insert(2);
        let u = a.union(b);
        assert_eq!(u.len(), 3);
        assert!(u.contains(0));
        assert!(u.contains(1));
        assert!(u.contains(2));
    }

    #[test]
    fn test_union_disjoint() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        let mut b = UnorderedCards::new();
        b.insert(2);
        b.insert(3);
        let u = a.union(b);
        assert_eq!(u.len(), 4);
        assert!(u.contains(0));
        assert!(u.contains(1));
        assert!(u.contains(2));
        assert!(u.contains(3));
    }

    #[test]
    fn test_union_with_empty() {
        let mut a = UnorderedCards::new();
        a.insert(5);
        a.insert(10);
        let empty = UnorderedCards::new();
        assert_eq!(a.union(empty), a);
    }

    #[test]
    fn test_intersection() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        a.insert(2);
        let mut b = UnorderedCards::new();
        b.insert(1);
        b.insert(2);
        b.insert(3);
        let i = a.intersection(b);
        assert_eq!(i.len(), 2);
        assert!(i.contains(1));
        assert!(i.contains(2));
    }

    #[test]
    fn test_intersection_disjoint() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        let mut b = UnorderedCards::new();
        b.insert(2);
        b.insert(3);
        let i = a.intersection(b);
        assert!(i.is_empty());
    }

    #[test]
    fn test_intersection_with_empty() {
        let mut a = UnorderedCards::new();
        a.insert(5);
        a.insert(10);
        let empty = UnorderedCards::new();
        assert_eq!(a.intersection(empty), UnorderedCards::new());
    }

    #[test]
    fn test_difference() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        a.insert(2);
        let mut b = UnorderedCards::new();
        b.insert(1);
        b.insert(2);
        b.insert(3);
        let d = a.difference(b);
        assert_eq!(d.len(), 1);
        assert!(d.contains(0));
    }

    #[test]
    fn test_difference_disjoint() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        let mut b = UnorderedCards::new();
        b.insert(2);
        b.insert(3);
        let d = a.difference(b);
        assert_eq!(d, a);
    }

    #[test]
    fn test_difference_from_empty() {
        let empty = UnorderedCards::new();
        let mut b = UnorderedCards::new();
        b.insert(1);
        b.insert(2);
        let d = empty.difference(b);
        assert!(d.is_empty());
    }

    #[test]
    fn test_difference_of_superset() {
        let mut a = UnorderedCards::new();
        a.insert(0);
        a.insert(1);
        let mut b = UnorderedCards::new();
        b.insert(0);
        b.insert(1);
        b.insert(2);
        let d = a.difference(b);
        assert!(d.is_empty());
    }

    // ── Iterator ──

    #[test]
    fn test_iter_empty() {
        let s = UnorderedCards::new();
        assert_eq!(s.iter().collect::<Vec<_>>(), Vec::<u8>::new());
    }

    #[test]
    fn test_iter_elements() {
        let mut s = UnorderedCards::new();
        s.insert(3);
        s.insert(7);
        s.insert(100);
        let elems: Vec<u8> = s.iter().collect();
        assert_eq!(elems, vec![3, 7, 100]);
    }

    #[test]
    fn test_iter_all_low_bits() {
        let mut s = UnorderedCards::new();
        for i in 0..8 {
            s.insert(i);
        }
        let elems: Vec<u8> = s.iter().collect();
        assert_eq!(elems, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_iter_size_hint() {
        let mut s = UnorderedCards::new();
        s.insert(1);
        s.insert(50);
        s.insert(100);
        let mut iter = s.iter();
        assert_eq!(iter.size_hint(), (3, Some(3)));
        iter.next();
        assert_eq!(iter.size_hint(), (2, Some(2)));
        iter.next();
        assert_eq!(iter.size_hint(), (1, Some(1)));
        iter.next();
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn test_iter_exact_size() {
        let mut s = UnorderedCards::new();
        s.insert(10);
        s.insert(20);
        s.insert(30);
        let mut iter = s.iter();
        assert_eq!(iter.len(), 3);
        iter.next();
        assert_eq!(iter.len(), 2);
        iter.next();
        assert_eq!(iter.len(), 1);
        iter.next();
        assert_eq!(iter.len(), 0);
    }

    // ── Random operations (seeded RNG) ──

    #[test]
    fn test_pick_random_empty() {
        let s = UnorderedCards::new();
        let mut rng = SmallRng::seed_from_u64(42);
        assert_eq!(s.pick_random(&mut rng), None);
    }

    #[test]
    fn test_pick_random_single() {
        let mut s = UnorderedCards::new();
        s.insert(99);
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..10 {
            assert_eq!(s.pick_random(&mut rng), Some(99));
        }
    }

    #[test]
    fn test_pick_random_does_not_modify() {
        let mut s = UnorderedCards::new();
        s.insert(1);
        s.insert(2);
        s.insert(3);
        let mut rng = SmallRng::seed_from_u64(42);
        let original_len = s.len();
        for _ in 0..20 {
            s.pick_random(&mut rng);
        }
        assert_eq!(s.len(), original_len);
    }

    #[test]
    fn test_pick_random_returns_member() {
        let mut s = UnorderedCards::new();
        for i in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
            s.insert(i);
        }
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            let picked = s.pick_random(&mut rng).unwrap();
            assert!(s.contains(picked), "picked {} not in set", picked);
        }
    }

    #[test]
    fn test_draw_empty() {
        let mut s = UnorderedCards::new();
        let mut rng = SmallRng::seed_from_u64(42);
        assert_eq!(s.draw(&mut rng), None);
    }

    #[test]
    fn test_draw_single() {
        let mut s = UnorderedCards::new();
        s.insert(55);
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw(&mut rng);
        assert_eq!(drawn, Some(55));
        assert!(s.is_empty());
    }

    #[test]
    fn test_draw_removes_element() {
        let mut s = UnorderedCards::new();
        s.insert(10);
        s.insert(20);
        s.insert(30);
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw(&mut rng).unwrap();
        assert_eq!(s.len(), 2);
        assert!(!s.contains(drawn));
    }

    #[test]
    fn test_draw_all() {
        let mut s = UnorderedCards::new();
        let elements: Vec<u8> = vec![5, 15, 25, 35, 45];
        for &e in &elements {
            s.insert(e);
        }
        let original = s;
        let mut rng = SmallRng::seed_from_u64(42);
        let mut drawn = Vec::new();
        while let Some(e) = s.draw(&mut rng) {
            drawn.push(e);
        }
        assert_eq!(drawn.len(), elements.len());
        for &d in &drawn {
            assert!(original.contains(d));
        }
    }

    #[test]
    fn test_draw_multiple_zero() {
        let mut s = UnorderedCards::new();
        s.insert(1);
        s.insert(2);
        s.insert(3);
        let original = s;
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_multiple(0, &mut rng);
        assert!(drawn.is_empty());
        assert_eq!(s, original);
    }

    #[test]
    fn test_draw_multiple_all() {
        let mut s = UnorderedCards::new();
        s.insert(0);
        s.insert(1);
        s.insert(2);
        s.insert(3);
        let n = s.len();
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_multiple(n, &mut rng);
        assert_eq!(drawn.len(), n);
        assert!(s.is_empty());
    }

    #[test]
    fn test_draw_multiple_more_than_available() {
        let mut s = UnorderedCards::new();
        s.insert(10);
        s.insert(20);
        let original = s;
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_multiple(100, &mut rng);
        assert_eq!(drawn, original);
        assert!(s.is_empty());
    }

    #[test]
    fn test_draw_multiple_subset() {
        let mut s = UnorderedCards::new();
        for i in 0..10 {
            s.insert(i);
        }
        let original = s;
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_multiple(4, &mut rng);
        assert_eq!(drawn.len(), 4);
        // drawn and remainder are disjoint
        assert!(drawn.intersection(s).is_empty());
        // union equals original
        assert_eq!(drawn.union(s), original);
    }

    #[test]
    fn test_draw_multiple_distribution() {
        use std::collections::HashMap;
        let base = {
            let mut s = UnorderedCards::new();
            for i in 0u8..5 {
                s.insert(i);
            }
            s
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let mut counts: HashMap<u128, u32> = HashMap::new();
        for _ in 0..10_000 {
            let mut s = base;
            let drawn = s.draw_multiple(2, &mut rng);
            *counts.entry(drawn.0).or_insert(0) += 1;
        }
        // C(5,2) = 10 possible subsets
        assert_eq!(counts.len(), 10);
        for (&_subset, &count) in &counts {
            assert!(
                count >= 500,
                "subset appeared only {} times (expected ~1000)",
                count
            );
        }
    }

    #[test]
    fn test_draw_up_to_zero() {
        let mut s = UnorderedCards::new();
        s.insert(1);
        s.insert(2);
        let original = s;
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_up_to(0, &mut rng);
        assert!(drawn.is_empty());
        assert_eq!(s, original);
    }

    #[test]
    fn test_draw_up_to_returns_valid_subset() {
        let mut s = UnorderedCards::new();
        for i in 0..8 {
            s.insert(i);
        }
        let original = s;
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_up_to(5, &mut rng);
        assert!(drawn.len() <= 5);
        // drawn is a subset of original
        assert_eq!(drawn.intersection(original), drawn);
        // source reduced by drawn count
        assert_eq!(s.len() + drawn.len(), original.len());
    }

    #[test]
    fn test_draw_up_to_max_exceeds_size() {
        let mut s = UnorderedCards::new();
        s.insert(10);
        s.insert(20);
        s.insert(30);
        let mut rng = SmallRng::seed_from_u64(42);
        let drawn = s.draw_up_to(100, &mut rng);
        assert!(drawn.len() <= 3);
    }

    #[test]
    fn test_draw_up_to_can_return_zero() {
        let base = {
            let mut s = UnorderedCards::new();
            for i in 0u8..5 {
                s.insert(i);
            }
            s
        };
        let mut rng = SmallRng::seed_from_u64(42);
        let mut saw_empty = false;
        for _ in 0..1000 {
            let mut s = base;
            let drawn = s.draw_up_to(2, &mut rng);
            if drawn.is_empty() {
                saw_empty = true;
                break;
            }
        }
        assert!(saw_empty, "draw_up_to never returned empty in 1000 trials");
    }

    // ── BINOM table ──

    #[test]
    fn test_binom_base_cases() {
        for n in [0, 1, 10, 50, 128] {
            assert_eq!(BINOM[n][0], 1, "BINOM[{}][0] should be 1", n);
        }
        for k in 1..10 {
            assert_eq!(BINOM[0][k], 0, "BINOM[0][{}] should be 0", k);
        }
    }

    #[test]
    fn test_binom_known_values() {
        assert_eq!(BINOM[5][2], 10);
        assert_eq!(BINOM[10][3], 120);
        assert_eq!(BINOM[128][1], 128);
    }

    #[test]
    fn test_binom_symmetry_small() {
        // BINOM[8][3] should equal BINOM[8][5] since C(8,3) = C(8,5)
        assert_eq!(BINOM[8][3], BINOM[8][5]);
        // BINOM[6][2] should equal BINOM[6][4] since C(6,2) = C(6,4)
        assert_eq!(BINOM[6][2], BINOM[6][4]);
        // BINOM[9][4] should equal BINOM[9][5] since C(9,4) = C(9,5)
        assert_eq!(BINOM[9][4], BINOM[9][5]);
    }

    // ── Serde round-trip ──

    #[test]
    fn test_serde_roundtrip_cards() {
        let mut registry = [Card::BasicRed; 128];
        registry[0] = Card::BasicRed;
        registry[5] = Card::BasicYellow;
        registry[10] = Card::BasicBlue;
        set_card_registry(&registry);

        let mut s = UnorderedCards::new();
        s.insert(0);
        s.insert(5);
        s.insert(10);

        let json = serde_json::to_string(&s).unwrap();
        let deserialized: UnorderedCards = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_empty() {
        set_card_registry(&[Card::BasicRed; 128]);
        let s = UnorderedCards::new();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "[]");
        let deserialized: UnorderedCards = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn test_serde_roundtrip_buyers() {
        let mut registry = [BuyerCard::Textiles2Vermilion; 128];
        registry[0] = BuyerCard::Textiles2Vermilion;
        registry[3] = BuyerCard::Textiles2Amber;
        registry[7] = BuyerCard::Textiles2Chartreuse;
        set_buyer_registry(&registry);

        let mut s = UnorderedBuyers::new();
        s.insert(0);
        s.insert(3);
        s.insert(7);

        let json = serde_json::to_string(&s).unwrap();
        let deserialized: UnorderedBuyers = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    // ── Equality/Clone ──

    #[test]
    fn test_equality() {
        let mut a = UnorderedCards::new();
        a.insert(5);
        a.insert(10);
        a.insert(15);
        let mut b = UnorderedCards::new();
        b.insert(15);
        b.insert(5);
        b.insert(10);
        assert_eq!(a, b);
    }

    #[test]
    fn test_inequality() {
        let mut a = UnorderedCards::new();
        a.insert(1);
        a.insert(2);
        let mut b = UnorderedCards::new();
        b.insert(1);
        b.insert(3);
        assert_ne!(a, b);
    }

    #[test]
    fn test_clone() {
        let mut original = UnorderedCards::new();
        original.insert(10);
        original.insert(20);
        let mut cloned = original;
        assert_eq!(original, cloned);
        cloned.insert(30);
        assert_ne!(original, cloned);
        assert!(!original.contains(30));
    }

    // ── BINOM_CUM table ──

    #[test]
    fn test_binom_cum_base() {
        // BINOM_CUM[n][0] should always equal 1 (= BINOM[n][0])
        for n in [0, 1, 10, 50, 128] {
            assert_eq!(BINOM_CUM[n][0], 1, "BINOM_CUM[{}][0] should be 1", n);
        }
    }

    #[test]
    fn test_binom_cum_values() {
        // BINOM_CUM[5][2] = C(5,0) + C(5,1) + C(5,2) = 1 + 5 + 10 = 16
        assert_eq!(BINOM_CUM[5][2], 16);
        // BINOM_CUM[n][n] for small n should be 2^n (sum of all binomials)
        // BINOM_CUM[5][5] = 2^5 = 32
        assert_eq!(BINOM_CUM[5][5], 32);
    }

    #[test]
    fn test_binom_cum_consistency() {
        // Verify BINOM_CUM[n][k] = sum of BINOM[n][0..=k] for a few values
        for n in [3, 7, 15, 30] {
            for k in 0..9usize.min(n) {
                let manual_sum: u64 = (0..=k).map(|j| BINOM[n][j]).sum();
                assert_eq!(BINOM_CUM[n][k], manual_sum, "BINOM_CUM[{}][{}]", n, k);
            }
        }
    }
}
