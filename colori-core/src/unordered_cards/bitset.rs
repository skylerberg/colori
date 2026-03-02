use std::marker::PhantomData;

use rand::Rng;
use rand::RngExt;

const BINOM: [[u64; 10]; 257] = {
    let mut table = [[0u64; 10]; 257];
    let mut n = 0usize;
    while n <= 256 {
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

const BINOM_CUM: [[u64; 10]; 257] = {
    let mut table = [[0u64; 10]; 257];
    let mut n = 0usize;
    while n <= 256 {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardMarker {}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuyerMarker {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitSet<T>(pub [u128; 2], PhantomData<T>);

pub type UnorderedCards = BitSet<CardMarker>;
pub type UnorderedBuyers = BitSet<BuyerMarker>;

impl<T> Default for BitSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BitSet<T> {
    #[inline]
    pub fn new() -> Self {
        BitSet([0; 2], PhantomData)
    }

    #[inline]
    pub fn insert(&mut self, id: u8) {
        let limb = (id >> 7) as usize;
        let bit = id & 127;
        self.0[limb] |= 1u128 << bit;
    }

    #[inline]
    pub fn remove(&mut self, id: u8) {
        let limb = (id >> 7) as usize;
        let bit = id & 127;
        self.0[limb] &= !(1u128 << bit);
    }

    #[inline]
    pub fn contains(&self, id: u8) -> bool {
        let limb = (id >> 7) as usize;
        let bit = id & 127;
        (self.0[limb] >> bit) & 1 != 0
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0[0] == 0 && self.0[1] == 0
    }

    #[inline]
    pub fn union(self, other: Self) -> Self {
        BitSet(
            [self.0[0] | other.0[0], self.0[1] | other.0[1]],
            PhantomData,
        )
    }

    #[inline]
    pub fn intersection(self, other: Self) -> Self {
        BitSet(
            [self.0[0] & other.0[0], self.0[1] & other.0[1]],
            PhantomData,
        )
    }

    #[inline]
    pub fn difference(self, other: Self) -> Self {
        BitSet(
            [self.0[0] & !other.0[0], self.0[1] & !other.0[1]],
            PhantomData,
        )
    }

    /// Returns the position of the lowest set bit, or None if empty.
    #[inline]
    pub fn lowest_bit(&self) -> Option<u8> {
        if self.0[0] != 0 {
            Some(self.0[0].trailing_zeros() as u8)
        } else if self.0[1] != 0 {
            Some(128 + self.0[1].trailing_zeros() as u8)
        } else {
            None
        }
    }

    #[inline]
    pub fn pick_random<R: Rng>(&self, rng: &mut R) -> Option<u8> {
        let count = self.len();
        if count == 0 {
            return None;
        }
        let k = rng.random_range(0..count);
        // Find the k-th set bit across both limbs
        let lo_count = self.0[0].count_ones();
        if k < lo_count {
            let mut val = self.0[0];
            for _ in 0..k {
                val &= val - 1;
            }
            Some(val.trailing_zeros() as u8)
        } else {
            let mut val = self.0[1];
            for _ in 0..(k - lo_count) {
                val &= val - 1;
            }
            Some(128 + val.trailing_zeros() as u8)
        }
    }

    #[inline]
    pub fn draw<R: Rng>(&mut self, rng: &mut R) -> Option<u8> {
        let count = self.len();
        if count == 0 {
            return None;
        }
        let k = rng.random_range(0..count);
        let lo_count = self.0[0].count_ones();
        let pos = if k < lo_count {
            let mut val = self.0[0];
            for _ in 0..k {
                val &= val - 1;
            }
            val.trailing_zeros() as u8
        } else {
            let mut val = self.0[1];
            for _ in 0..(k - lo_count) {
                val &= val - 1;
            }
            128 + val.trailing_zeros() as u8
        };
        self.remove(pos);
        Some(pos)
    }

    #[inline]
    pub fn draw_multiple<R: Rng>(&mut self, count: u32, rng: &mut R) -> Self {
        let n = self.len();
        if count == 0 {
            return Self::new();
        }
        if count >= n {
            let all = self.0;
            self.0 = [0; 2];
            return BitSet(all, PhantomData);
        }
        let mut remaining = n;
        let mut to_pick = count;
        let mut selected = [0u128; 2];

        // Selection sampling (Algorithm S): for each element,
        // include with probability to_pick/remaining.
        let mut bits0 = self.0[0];
        while to_pick > 0 && bits0 != 0 {
            if remaining == to_pick {
                // Must take all remaining elements
                selected[0] |= bits0;
                selected[1] = self.0[1];
                self.0[0] &= !selected[0];
                self.0[1] = 0;
                return BitSet(selected, PhantomData);
            }
            let pos = bits0.trailing_zeros();
            bits0 &= bits0 - 1;
            if rng.random_range(0..remaining) < to_pick {
                selected[0] |= 1u128 << pos;
                to_pick -= 1;
            }
            remaining -= 1;
        }
        let mut bits1 = self.0[1];
        while to_pick > 0 && bits1 != 0 {
            if remaining == to_pick {
                selected[1] |= bits1;
                self.0[0] &= !selected[0];
                self.0[1] &= !selected[1];
                return BitSet(selected, PhantomData);
            }
            let pos = bits1.trailing_zeros();
            bits1 &= bits1 - 1;
            if rng.random_range(0..remaining) < to_pick {
                selected[1] |= 1u128 << pos;
                to_pick -= 1;
            }
            remaining -= 1;
        }

        self.0[0] &= !selected[0];
        self.0[1] &= !selected[1];
        BitSet(selected, PhantomData)
    }

    #[inline]
    pub fn draw_up_to<R: Rng>(&mut self, max_count: u8, rng: &mut R) -> Self {
        if max_count == 1 {
            let n = self.len();
            if n == 0 {
                return Self::new();
            }
            let r = rng.random_range(0..(n as u64 + 1));
            if r == 0 {
                return Self::new();
            }
            // Find the (r-1)-th set bit
            let k = (r - 1) as u32;
            let lo_count = self.0[0].count_ones();
            let pos;
            let mut result = [0u128; 2];
            if k < lo_count {
                let mut val = self.0[0];
                for _ in 0..k {
                    val &= val - 1;
                }
                pos = val.trailing_zeros() as u8;
                result[0] = 1u128 << pos;
            } else {
                let mut val = self.0[1];
                for _ in 0..(k - lo_count) {
                    val &= val - 1;
                }
                pos = 128 + val.trailing_zeros() as u8;
                result[1] = 1u128 << (pos - 128);
            }
            self.remove(pos);
            return BitSet(result, PhantomData);
        }
        let n = self.len() as usize;
        let c = (max_count as usize).min(n);
        if c == 0 {
            return Self::new();
        }
        let total = BINOM_CUM[n][c];
        let r = rng.random_range(0..total);
        // Find which size bucket using cumulative table
        let mut size = 0usize;
        for k in 0..=c {
            if r < BINOM_CUM[n][k] {
                size = k;
                break;
            }
        }
        if size == 0 {
            return Self::new();
        }
        // Selection sampling for `size` elements
        let mut remaining = n as u32;
        let mut to_pick = size as u32;
        let mut selected = [0u128; 2];

        let mut bits0 = self.0[0];
        while to_pick > 0 && bits0 != 0 {
            if remaining == to_pick {
                selected[0] |= bits0;
                selected[1] = self.0[1];
                self.0[0] &= !selected[0];
                self.0[1] = 0;
                return BitSet(selected, PhantomData);
            }
            let pos = bits0.trailing_zeros();
            bits0 &= bits0 - 1;
            if rng.random_range(0..remaining) < to_pick {
                selected[0] |= 1u128 << pos;
                to_pick -= 1;
            }
            remaining -= 1;
        }
        let mut bits1 = self.0[1];
        while to_pick > 0 && bits1 != 0 {
            if remaining == to_pick {
                selected[1] |= bits1;
                self.0[0] &= !selected[0];
                self.0[1] &= !selected[1];
                return BitSet(selected, PhantomData);
            }
            let pos = bits1.trailing_zeros();
            bits1 &= bits1 - 1;
            if rng.random_range(0..remaining) < to_pick {
                selected[1] |= 1u128 << pos;
                to_pick -= 1;
            }
            remaining -= 1;
        }

        self.0[0] &= !selected[0];
        self.0[1] &= !selected[1];
        BitSet(selected, PhantomData)
    }

    #[inline]
    pub fn iter(self) -> BitIter {
        BitIter {
            lo: self.0[0],
            hi: self.0[1],
        }
    }
}

pub struct BitIter {
    lo: u128,
    hi: u128,
}

impl Iterator for BitIter {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.lo != 0 {
            let pos = self.lo.trailing_zeros() as u8;
            self.lo &= self.lo - 1;
            Some(pos)
        } else if self.hi != 0 {
            let pos = self.hi.trailing_zeros() as u8;
            self.hi &= self.hi - 1;
            Some(128 + pos)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let c = (self.lo.count_ones() + self.hi.count_ones()) as usize;
        (c, Some(c))
    }
}

impl ExactSizeIterator for BitIter {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use wyrand::WyRand;

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
    fn test_insert_high_ids() {
        let mut s = UnorderedCards::new();
        s.insert(128);
        s.insert(200);
        s.insert(255);
        assert_eq!(s.len(), 3);
        assert!(s.contains(128));
        assert!(s.contains(200));
        assert!(s.contains(255));
        assert!(!s.contains(127));
        assert!(!s.contains(0));
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
    fn test_remove_high_id() {
        let mut s = UnorderedCards::new();
        s.insert(200);
        s.remove(200);
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
        assert!(!s.contains(128));
        assert!(!s.contains(255));
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
    fn test_union_across_limbs() {
        let mut a = UnorderedCards::new();
        a.insert(50);
        a.insert(200);
        let mut b = UnorderedCards::new();
        b.insert(100);
        b.insert(250);
        let u = a.union(b);
        assert_eq!(u.len(), 4);
        assert!(u.contains(50));
        assert!(u.contains(100));
        assert!(u.contains(200));
        assert!(u.contains(250));
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
    fn test_iter_across_limbs() {
        let mut s = UnorderedCards::new();
        s.insert(50);
        s.insert(127);
        s.insert(128);
        s.insert(200);
        let elems: Vec<u8> = s.iter().collect();
        assert_eq!(elems, vec![50, 127, 128, 200]);
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
        let mut rng = WyRand::seed_from_u64(42);
        assert_eq!(s.pick_random(&mut rng), None);
    }

    #[test]
    fn test_pick_random_single() {
        let mut s = UnorderedCards::new();
        s.insert(99);
        let mut rng = WyRand::seed_from_u64(42);
        for _ in 0..10 {
            assert_eq!(s.pick_random(&mut rng), Some(99));
        }
    }

    #[test]
    fn test_pick_random_high_id() {
        let mut s = UnorderedCards::new();
        s.insert(200);
        let mut rng = WyRand::seed_from_u64(42);
        for _ in 0..10 {
            assert_eq!(s.pick_random(&mut rng), Some(200));
        }
    }

    #[test]
    fn test_pick_random_does_not_modify() {
        let mut s = UnorderedCards::new();
        s.insert(1);
        s.insert(2);
        s.insert(3);
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
        for _ in 0..100 {
            let picked = s.pick_random(&mut rng).unwrap();
            assert!(s.contains(picked), "picked {} not in set", picked);
        }
    }

    #[test]
    fn test_draw_empty() {
        let mut s = UnorderedCards::new();
        let mut rng = WyRand::seed_from_u64(42);
        assert_eq!(s.draw(&mut rng), None);
    }

    #[test]
    fn test_draw_single() {
        let mut s = UnorderedCards::new();
        s.insert(55);
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
        let mut counts: HashMap<UnorderedCards, u32> = HashMap::new();
        for _ in 0..10_000 {
            let mut s = base;
            let drawn = s.draw_multiple(2, &mut rng);
            *counts.entry(drawn).or_insert(0) += 1;
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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
        let mut rng = WyRand::seed_from_u64(42);
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

    // ── lowest_bit ──

    #[test]
    fn test_lowest_bit_empty() {
        let s = UnorderedCards::new();
        assert_eq!(s.lowest_bit(), None);
    }

    #[test]
    fn test_lowest_bit_lo() {
        let mut s = UnorderedCards::new();
        s.insert(42);
        s.insert(100);
        assert_eq!(s.lowest_bit(), Some(42));
    }

    #[test]
    fn test_lowest_bit_hi_only() {
        let mut s = UnorderedCards::new();
        s.insert(200);
        s.insert(250);
        assert_eq!(s.lowest_bit(), Some(200));
    }

    // ── BINOM table ──

    #[test]
    fn test_binom_base_cases() {
        for n in [0, 1, 10, 50, 128, 256] {
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
        assert_eq!(BINOM[256][1], 256);
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
        for n in [0, 1, 10, 50, 128, 256] {
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
