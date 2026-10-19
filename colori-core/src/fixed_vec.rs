use std::fmt;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

pub struct FixedVec<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> FixedVec<T, N> {
    pub fn new() -> Self {
        FixedVec {
            // SAFETY: An uninitialized `[MaybeUninit<_>; N]` is valid.
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        assert!(self.len < N, "FixedVec overflow: capacity is {N}");
        self.data[self.len] = MaybeUninit::new(value);
        self.len += 1;
    }

    /// Remove the element at `index`, shifting the rest down. Use when order
    /// carries meaning; prefer `swap_remove` when it does not.
    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "remove index out of bounds");
        // SAFETY: `index` is within the initialized range, so the read leaves
        // a hole that the shift immediately fills; the tail slot is then
        // logically uninitialized and `len` is decremented to match.
        unsafe {
            let ptr = self.data.as_mut_ptr();
            let removed = (*ptr.add(index)).assume_init_read();
            std::ptr::copy(ptr.add(index + 1), ptr.add(index), self.len - index - 1);
            self.len -= 1;
            removed
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "swap_remove index out of bounds");
        self.len -= 1;
        if index != self.len {
            // SAFETY: Both index and self.len are within initialized range.
            unsafe {
                let ptr = self.data.as_mut_ptr();
                std::ptr::swap(ptr.add(index), ptr.add(self.len));
            }
        }
        // SAFETY: The element at self.len is initialized (either original or swapped).
        unsafe { self.data[self.len].assume_init_read() }
    }

    pub fn from_slice(slice: &[T]) -> Self
    where
        T: Clone,
    {
        assert!(slice.len() <= N, "slice too large for FixedVec<_, {N}>");
        let mut fv = Self::new();
        for item in slice {
            fv.push(item.clone());
        }
        fv
    }
}

impl<T, const N: usize> Deref for FixedVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        // SAFETY: Elements 0..self.len are initialized.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len) }
    }
}

impl<T, const N: usize> DerefMut for FixedVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: Elements 0..self.len are initialized.
        unsafe { std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len) }
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a FixedVec<T, N> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deref().iter()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut FixedVec<T, N> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deref_mut().iter_mut()
    }
}

impl<T: Clone, const N: usize> Clone for FixedVec<T, N> {
    fn clone(&self) -> Self {
        let mut new = Self::new();
        for item in self.iter() {
            new.push(item.clone());
        }
        new
    }

    fn clone_from(&mut self, source: &Self) {
        // Reuse existing elements' clone_from where possible (important for
        // PlayerState inner Vecs to reuse their heap allocations).
        let common = self.len.min(source.len);

        // clone_from for overlapping elements
        for i in 0..common {
            // SAFETY: Elements 0..common are initialized in both self and source.
            unsafe {
                let dst = &mut *self.data[i].as_mut_ptr();
                let src = &*source.data[i].as_ptr();
                dst.clone_from(src);
            }
        }

        if source.len > common {
            // Clone additional elements from source
            for i in common..source.len {
                // SAFETY: source elements are initialized.
                self.data[i] = MaybeUninit::new(unsafe { &*source.data[i].as_ptr() }.clone());
            }
        } else if self.len > common {
            // Drop excess elements
            for i in common..self.len {
                // SAFETY: These elements are initialized and need to be dropped.
                unsafe { self.data[i].assume_init_drop() };
            }
        }

        self.len = source.len;
    }
}

impl<T, const N: usize> Drop for FixedVec<T, N> {
    fn drop(&mut self) {
        for i in 0..self.len {
            // SAFETY: Elements 0..self.len are initialized.
            unsafe { self.data[i].assume_init_drop() };
        }
    }
}

impl<T, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> FromIterator<T> for FixedVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut fv = Self::new();
        for item in iter {
            fv.push(item);
        }
        fv
    }
}

impl<T: Serialize, const N: usize> Serialize for FixedVec<T, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for FixedVec<T, N> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Vec::<T>::deserialize(deserializer)?;
        if v.len() > N {
            return Err(serde::de::Error::custom(format!(
                "array length {} exceeds FixedVec capacity {N}",
                v.len()
            )));
        }
        Ok(v.into_iter().collect())
    }
}

impl<T: PartialEq, const N: usize> PartialEq for FixedVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: Eq, const N: usize> Eq for FixedVec<T, N> {}

impl<T: fmt::Debug, const N: usize> fmt::Debug for FixedVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

/// A [`FixedVec`] of `Copy` items that is itself `Copy`, so a value holding
/// one — a [`crate::types::Choice`] — has no drop glue and clones by memcpy.
#[derive(Clone, Copy)]
pub struct CopyVec<T: Copy, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: u8,
}

impl<T: Copy, const N: usize> CopyVec<T, N> {
    pub fn new() -> Self {
        CopyVec {
            data: [MaybeUninit::uninit(); N],
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) {
        assert!((self.len as usize) < N, "CopyVec overflow: capacity is {N}");
        self.data[self.len as usize] = MaybeUninit::new(value);
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: the element at the old last index was initialized.
        Some(unsafe { self.data[self.len as usize].assume_init() })
    }

    pub fn truncate(&mut self, len: usize) {
        self.len = self.len.min(len as u8);
    }

    pub fn from_slice(slice: &[T]) -> Self {
        slice.iter().copied().collect()
    }
}

impl<T: Copy, const N: usize> Deref for CopyVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        // SAFETY: Elements 0..self.len are initialized.
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len as usize) }
    }
}

impl<T: Copy, const N: usize> DerefMut for CopyVec<T, N> {
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: Elements 0..self.len are initialized.
        unsafe { std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len as usize) }
    }
}

impl<'a, T: Copy, const N: usize> IntoIterator for &'a CopyVec<T, N> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.deref().iter()
    }
}

impl<T: Copy, const N: usize> Default for CopyVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> FromIterator<T> for CopyVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut v = Self::new();
        v.extend(iter);
        v
    }
}

impl<T: Copy, const N: usize> Extend<T> for CopyVec<T, N> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item);
        }
    }
}

impl<T: Copy + Serialize, const N: usize> Serialize for CopyVec<T, N> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de, T: Copy + Deserialize<'de>, const N: usize> Deserialize<'de> for CopyVec<T, N> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Vec::<T>::deserialize(deserializer)?;
        if v.len() > N {
            return Err(serde::de::Error::custom(format!(
                "array length {} exceeds CopyVec capacity {N}",
                v.len()
            )));
        }
        Ok(v.into_iter().collect())
    }
}

impl<T: Copy + PartialEq, const N: usize> PartialEq for CopyVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: Copy + Eq, const N: usize> Eq for CopyVec<T, N> {}

impl<T: Copy + std::hash::Hash, const N: usize> std::hash::Hash for CopyVec<T, N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<T: Copy + fmt::Debug, const N: usize> fmt::Debug for CopyVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}
