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

impl<T: fmt::Debug, const N: usize> fmt::Debug for FixedVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}
