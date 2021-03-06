// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Slice management and manipulation
//!
//! For more details `std::slice`.

#![stable]
#![doc(primitive = "slice")]

// How this module is organized.
//
// The library infrastructure for slices is fairly messy. There's
// a lot of stuff defined here. Let's keep it clean.
//
// Since slices don't support inherent methods; all operations
// on them are defined on traits, which are then reexported from
// the prelude for convenience. So there are a lot of traits here.
//
// The layout of this file is thus:
//
// * Slice-specific 'extension' traits and their implementations. This
//   is where most of the slice API resides.
// * Implementations of a few common traits with important slice ops.
// * Definitions of a bunch of iterators.
// * Free functions.
// * The `raw` and `bytes` submodules.
// * Boilerplate trait implementations.

pub use self::BinarySearchResult::*;

use mem::transmute;
use clone::Clone;
use cmp::{PartialEq, PartialOrd, Eq, Ord, Ordering, Less, Equal, Greater, Equiv};
use cmp;
use default::Default;
use iter::*;
use num::Int;
use ops;
use option::{None, Option, Some};
use ptr;
use ptr::RawPtr;
use mem;
use mem::size_of;
use kinds::{Sized, marker};
use raw::Repr;
// Avoid conflicts with *both* the Slice trait (buggy) and the `slice::raw` module.
use raw::Slice as RawSlice;


//
// Extension traits
//

/// Extension methods for slices.
#[unstable = "may merge with other traits"]
pub trait SlicePrelude<T> for Sized? {
    /// Returns a subslice spanning the interval [`start`, `end`).
    ///
    /// Panics when the end of the new slice lies beyond the end of the
    /// original slice (i.e. when `end > self.len()`) or when `start > end`.
    ///
    /// Slicing with `start` equal to `end` yields an empty slice.
    #[unstable = "waiting on final error conventions/slicing syntax"]
    fn slice<'a>(&'a self, start: uint, end: uint) -> &'a [T];

    /// Returns a subslice from `start` to the end of the slice.
    ///
    /// Panics when `start` is strictly greater than the length of the original slice.
    ///
    /// Slicing from `self.len()` yields an empty slice.
    #[unstable = "waiting on final error conventions/slicing syntax"]
    fn slice_from<'a>(&'a self, start: uint) -> &'a [T];

    /// Returns a subslice from the start of the slice to `end`.
    ///
    /// Panics when `end` is strictly greater than the length of the original slice.
    ///
    /// Slicing to `0` yields an empty slice.
    #[unstable = "waiting on final error conventions/slicing syntax"]
    fn slice_to<'a>(&'a self, end: uint) -> &'a [T];

    /// Divides one slice into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// Panics if `mid > len`.
    #[unstable = "waiting on final error conventions"]
    fn split_at<'a>(&'a self, mid: uint) -> (&'a [T], &'a [T]);

    /// Returns an iterator over the slice
    #[unstable = "iterator type may change"]
    fn iter<'a>(&'a self) -> Items<'a, T>;

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`.  The matched element is not contained in the subslices.
    #[unstable = "iterator type may change, waiting on unboxed closures"]
    fn split<'a>(&'a self, pred: |&T|: 'a -> bool) -> Splits<'a, T>;

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`, limited to splitting at most `n` times.  The matched element is
    /// not contained in the subslices.
    #[unstable = "iterator type may change"]
    fn splitn<'a>(&'a self, n: uint, pred: |&T|: 'a -> bool) -> SplitsN<Splits<'a, T>>;

    /// Returns an iterator over subslices separated by elements that match
    /// `pred` limited to splitting at most `n` times. This starts at the end of
    /// the slice and works backwards.  The matched element is not contained in
    /// the subslices.
    #[unstable = "iterator type may change"]
    fn rsplitn<'a>(&'a self,  n: uint, pred: |&T|: 'a -> bool) -> SplitsN<Splits<'a, T>>;

    /// Returns an iterator over all contiguous windows of length
    /// `size`. The windows overlap. If the slice is shorter than
    /// `size`, the iterator returns no values.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    ///
    /// # Example
    ///
    /// Print the adjacent pairs of a slice (i.e. `[1,2]`, `[2,3]`,
    /// `[3,4]`):
    ///
    /// ```rust
    /// let v = &[1i, 2, 3, 4];
    /// for win in v.windows(2) {
    ///     println!("{}", win);
    /// }
    /// ```
    #[unstable = "iterator type may change"]
    fn windows<'a>(&'a self, size: uint) -> Windows<'a, T>;

    /// Returns an iterator over `size` elements of the slice at a
    /// time. The chunks do not overlap. If `size` does not divide the
    /// length of the slice, then the last chunk will not have length
    /// `size`.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    ///
    /// # Example
    ///
    /// Print the slice two elements at a time (i.e. `[1,2]`,
    /// `[3,4]`, `[5]`):
    ///
    /// ```rust
    /// let v = &[1i, 2, 3, 4, 5];
    /// for win in v.chunks(2) {
    ///     println!("{}", win);
    /// }
    /// ```
    #[unstable = "iterator type may change"]
    fn chunks<'a>(&'a self, size: uint) -> Chunks<'a, T>;

    /// Returns the element of a slice at the given index, or `None` if the
    /// index is out of bounds.
    #[unstable = "waiting on final collection conventions"]
    fn get<'a>(&'a self, index: uint) -> Option<&'a T>;

    /// Returns the first element of a slice, or `None` if it is empty.
    #[unstable = "name may change"]
    fn head<'a>(&'a self) -> Option<&'a T>;

    /// Returns all but the first element of a slice.
    #[unstable = "name may change"]
    fn tail<'a>(&'a self) -> &'a [T];

    /// Returns all but the last element of a slice.
    #[unstable = "name may change"]
    fn init<'a>(&'a self) -> &'a [T];

    /// Returns the last element of a slice, or `None` if it is empty.
    #[unstable = "name may change"]
    fn last<'a>(&'a self) -> Option<&'a T>;

    /// Returns a pointer to the element at the given index, without doing
    /// bounds checking.
    #[unstable]
    unsafe fn unsafe_get<'a>(&'a self, index: uint) -> &'a T;

    /// Returns an unsafe pointer to the slice's buffer
    ///
    /// The caller must ensure that the slice outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    ///
    /// Modifying the slice may cause its buffer to be reallocated, which
    /// would also make any pointers to it invalid.
    #[unstable]
    fn as_ptr(&self) -> *const T;

    /// Binary search a sorted slice with a comparator function.
    ///
    /// The comparator function should implement an order consistent
    /// with the sort order of the underlying slice, returning an
    /// order code that indicates whether its argument is `Less`,
    /// `Equal` or `Greater` the desired target.
    ///
    /// If a matching value is found then returns `Found`, containing
    /// the index for the matched element; if no match is found then
    /// `NotFound` is returned, containing the index where a matching
    /// element could be inserted while maintaining sorted order.
    ///
    /// # Example
    ///
    /// Looks up a series of four elements. The first is found, with a
    /// uniquely determined position; the second and third are not
    /// found; the fourth could match any position in `[1,4]`.
    ///
    /// ```rust
    /// use std::slice::{Found, NotFound};
    /// let s = [0i, 1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
    /// let s = s.as_slice();
    ///
    /// let seek = 13;
    /// assert_eq!(s.binary_search(|probe| probe.cmp(&seek)), Found(9));
    /// let seek = 4;
    /// assert_eq!(s.binary_search(|probe| probe.cmp(&seek)), NotFound(7));
    /// let seek = 100;
    /// assert_eq!(s.binary_search(|probe| probe.cmp(&seek)), NotFound(13));
    /// let seek = 1;
    /// let r = s.binary_search(|probe| probe.cmp(&seek));
    /// assert!(match r { Found(1...4) => true, _ => false, });
    /// ```
    #[unstable = "waiting on unboxed closures"]
    fn binary_search(&self, f: |&T| -> Ordering) -> BinarySearchResult;

    /// Return the number of elements in the slice
    ///
    /// # Example
    ///
    /// ```
    /// let a = [1i, 2, 3];
    /// assert_eq!(a.len(), 3);
    /// ```
    #[experimental = "not triaged yet"]
    fn len(&self) -> uint;

    /// Returns true if the slice has a length of 0
    ///
    /// # Example
    ///
    /// ```
    /// let a = [1i, 2, 3];
    /// assert!(!a.is_empty());
    /// ```
    #[inline]
    #[experimental = "not triaged yet"]
    fn is_empty(&self) -> bool { self.len() == 0 }
    /// Returns a mutable reference to the element at the given index,
    /// or `None` if the index is out of bounds
    #[unstable = "waiting on final error conventions"]
    fn get_mut<'a>(&'a mut self, index: uint) -> Option<&'a mut T>;

    /// Work with `self` as a mut slice.
    /// Primarily intended for getting a &mut [T] from a [T, ..N].
    fn as_mut_slice<'a>(&'a mut self) -> &'a mut [T];

    /// Returns a mutable subslice spanning the interval [`start`, `end`).
    ///
    /// Panics when the end of the new slice lies beyond the end of the
    /// original slice (i.e. when `end > self.len()`) or when `start > end`.
    ///
    /// Slicing with `start` equal to `end` yields an empty slice.
    #[unstable = "waiting on final error conventions"]
    fn slice_mut<'a>(&'a mut self, start: uint, end: uint) -> &'a mut [T];

    /// Returns a mutable subslice from `start` to the end of the slice.
    ///
    /// Panics when `start` is strictly greater than the length of the original slice.
    ///
    /// Slicing from `self.len()` yields an empty slice.
    #[unstable = "waiting on final error conventions"]
    fn slice_from_mut<'a>(&'a mut self, start: uint) -> &'a mut [T];

    /// Returns a mutable subslice from the start of the slice to `end`.
    ///
    /// Panics when `end` is strictly greater than the length of the original slice.
    ///
    /// Slicing to `0` yields an empty slice.
    #[unstable = "waiting on final error conventions"]
    fn slice_to_mut<'a>(&'a mut self, end: uint) -> &'a mut [T];

    /// Returns an iterator that allows modifying each value
    #[unstable = "waiting on iterator type name conventions"]
    fn iter_mut<'a>(&'a mut self) -> MutItems<'a, T>;

    /// Returns a mutable pointer to the first element of a slice, or `None` if it is empty
    #[unstable = "name may change"]
    fn head_mut<'a>(&'a mut self) -> Option<&'a mut T>;

    /// Returns all but the first element of a mutable slice
    #[unstable = "name may change"]
    fn tail_mut<'a>(&'a mut self) -> &'a mut [T];

    /// Returns all but the last element of a mutable slice
    #[unstable = "name may change"]
    fn init_mut<'a>(&'a mut self) -> &'a mut [T];

    /// Returns a mutable pointer to the last item in the slice.
    #[unstable = "name may change"]
    fn last_mut<'a>(&'a mut self) -> Option<&'a mut T>;

    /// Returns an iterator over mutable subslices separated by elements that
    /// match `pred`.  The matched element is not contained in the subslices.
    #[unstable = "waiting on unboxed closures, iterator type name conventions"]
    fn split_mut<'a>(&'a mut self, pred: |&T|: 'a -> bool) -> MutSplits<'a, T>;

    /// Returns an iterator over subslices separated by elements that match
    /// `pred`, limited to splitting at most `n` times.  The matched element is
    /// not contained in the subslices.
    #[unstable = "waiting on unboxed closures, iterator type name conventions"]
    fn splitn_mut<'a>(&'a mut self, n: uint, pred: |&T|: 'a -> bool) -> SplitsN<MutSplits<'a, T>>;

    /// Returns an iterator over subslices separated by elements that match
    /// `pred` limited to splitting at most `n` times. This starts at the end of
    /// the slice and works backwards.  The matched element is not contained in
    /// the subslices.
    #[unstable = "waiting on unboxed closures, iterator type name conventions"]
    fn rsplitn_mut<'a>(&'a mut self,  n: uint, pred: |&T|: 'a -> bool) -> SplitsN<MutSplits<'a, T>>;

    /// Returns an iterator over `chunk_size` elements of the slice at a time.
    /// The chunks are mutable and do not overlap. If `chunk_size` does
    /// not divide the length of the slice, then the last chunk will not
    /// have length `chunk_size`.
    ///
    /// # Panics
    ///
    /// Panics if `chunk_size` is 0.
    #[unstable = "waiting on iterator type name conventions"]
    fn chunks_mut<'a>(&'a mut self, chunk_size: uint) -> MutChunks<'a, T>;

    /// Swaps two elements in a slice.
    ///
    /// Panics if `a` or `b` are out of bounds.
    ///
    /// # Arguments
    ///
    /// * a - The index of the first element
    /// * b - The index of the second element
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut v = ["a", "b", "c", "d"];
    /// v.swap(1, 3);
    /// assert!(v == ["a", "d", "c", "b"]);
    /// ```
    #[unstable = "waiting on final error conventions"]
    fn swap(&mut self, a: uint, b: uint);

    /// Divides one `&mut` into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// Panics if `mid > len`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut v = [1i, 2, 3, 4, 5, 6];
    ///
    /// // scoped to restrict the lifetime of the borrows
    /// {
    ///    let (left, right) = v.split_at_mut(0);
    ///    assert!(left == &mut []);
    ///    assert!(right == &mut [1i, 2, 3, 4, 5, 6]);
    /// }
    ///
    /// {
    ///     let (left, right) = v.split_at_mut(2);
    ///     assert!(left == &mut [1i, 2]);
    ///     assert!(right == &mut [3i, 4, 5, 6]);
    /// }
    ///
    /// {
    ///     let (left, right) = v.split_at_mut(6);
    ///     assert!(left == &mut [1i, 2, 3, 4, 5, 6]);
    ///     assert!(right == &mut []);
    /// }
    /// ```
    #[unstable = "waiting on final error conventions"]
    fn split_at_mut<'a>(&'a mut self, mid: uint) -> (&'a mut [T], &'a mut [T]);

    /// Reverse the order of elements in a slice, in place.
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut v = [1i, 2, 3];
    /// v.reverse();
    /// assert!(v == [3i, 2, 1]);
    /// ```
    #[experimental = "may be moved to iterators instead"]
    fn reverse(&mut self);

    /// Returns an unsafe mutable pointer to the element in index
    #[experimental = "waiting on unsafe conventions"]
    unsafe fn unsafe_mut<'a>(&'a mut self, index: uint) -> &'a mut T;

    /// Return an unsafe mutable pointer to the slice's buffer.
    ///
    /// The caller must ensure that the slice outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    ///
    /// Modifying the slice may cause its buffer to be reallocated, which
    /// would also make any pointers to it invalid.
    #[inline]
    #[unstable]
    fn as_mut_ptr(&mut self) -> *mut T;
}

#[unstable]
impl<T> SlicePrelude<T> for [T] {
    #[inline]
    fn slice(&self, start: uint, end: uint) -> &[T] {
        assert!(start <= end);
        assert!(end <= self.len());
        unsafe {
            transmute(RawSlice {
                data: self.as_ptr().offset(start as int),
                len: (end - start)
            })
        }
    }

    #[inline]
    fn slice_from(&self, start: uint) -> &[T] {
        self.slice(start, self.len())
    }

    #[inline]
    fn slice_to(&self, end: uint) -> &[T] {
        self.slice(0, end)
    }

    #[inline]
    fn split_at(&self, mid: uint) -> (&[T], &[T]) {
        (self[..mid], self[mid..])
    }

    #[inline]
    fn iter<'a>(&'a self) -> Items<'a, T> {
        unsafe {
            let p = self.as_ptr();
            if mem::size_of::<T>() == 0 {
                Items{ptr: p,
                      end: (p as uint + self.len()) as *const T,
                      marker: marker::ContravariantLifetime::<'a>}
            } else {
                Items{ptr: p,
                      end: p.offset(self.len() as int),
                      marker: marker::ContravariantLifetime::<'a>}
            }
        }
    }

    #[inline]
    fn split<'a>(&'a self, pred: |&T|: 'a -> bool) -> Splits<'a, T> {
        Splits {
            v: self,
            pred: pred,
            finished: false
        }
    }

    #[inline]
    fn splitn<'a>(&'a self, n: uint, pred: |&T|: 'a -> bool) -> SplitsN<Splits<'a, T>> {
        SplitsN {
            iter: self.split(pred),
            count: n,
            invert: false
        }
    }

    #[inline]
    fn rsplitn<'a>(&'a self, n: uint, pred: |&T|: 'a -> bool) -> SplitsN<Splits<'a, T>> {
        SplitsN {
            iter: self.split(pred),
            count: n,
            invert: true
        }
    }

    #[inline]
    fn windows(&self, size: uint) -> Windows<T> {
        assert!(size != 0);
        Windows { v: self, size: size }
    }

    #[inline]
    fn chunks(&self, size: uint) -> Chunks<T> {
        assert!(size != 0);
        Chunks { v: self, size: size }
    }

    #[inline]
    fn get(&self, index: uint) -> Option<&T> {
        if index < self.len() { Some(&self[index]) } else { None }
    }

    #[inline]
    fn head(&self) -> Option<&T> {
        if self.len() == 0 { None } else { Some(&self[0]) }
    }

    #[inline]
    fn tail(&self) -> &[T] { self[1..] }

    #[inline]
    fn init(&self) -> &[T] {
        self[..self.len() - 1]
    }

    #[inline]
    fn last(&self) -> Option<&T> {
        if self.len() == 0 { None } else { Some(&self[self.len() - 1]) }
    }

    #[inline]
    unsafe fn unsafe_get(&self, index: uint) -> &T {
        transmute(self.repr().data.offset(index as int))
    }

    #[inline]
    fn as_ptr(&self) -> *const T {
        self.repr().data
    }

    #[unstable]
    fn binary_search(&self, f: |&T| -> Ordering) -> BinarySearchResult {
        let mut base : uint = 0;
        let mut lim : uint = self.len();

        while lim != 0 {
            let ix = base + (lim >> 1);
            match f(&self[ix]) {
                Equal => return Found(ix),
                Less => {
                    base = ix + 1;
                    lim -= 1;
                }
                Greater => ()
            }
            lim >>= 1;
        }
        return NotFound(base);
    }

    #[inline]
    fn len(&self) -> uint { self.repr().len }

    #[inline]
    fn get_mut(&mut self, index: uint) -> Option<&mut T> {
        if index < self.len() { Some(&mut self[index]) } else { None }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] { self }

    fn slice_mut(&mut self, start: uint, end: uint) -> &mut [T] {
        self[mut start..end]
    }

    #[inline]
    fn slice_from_mut(&mut self, start: uint) -> &mut [T] {
        self[mut start..]
    }

    #[inline]
    fn slice_to_mut(&mut self, end: uint) -> &mut [T] {
        self[mut ..end]
    }

    #[inline]
    fn split_at_mut(&mut self, mid: uint) -> (&mut [T], &mut [T]) {
        unsafe {
            let self2: &mut [T] = mem::transmute_copy(&self);
            (self[mut ..mid], self2[mut mid..])
        }
    }

    #[inline]
    fn iter_mut<'a>(&'a mut self) -> MutItems<'a, T> {
        unsafe {
            let p = self.as_mut_ptr();
            if mem::size_of::<T>() == 0 {
                MutItems{ptr: p,
                         end: (p as uint + self.len()) as *mut T,
                         marker: marker::ContravariantLifetime::<'a>,
                         marker2: marker::NoCopy}
            } else {
                MutItems{ptr: p,
                         end: p.offset(self.len() as int),
                         marker: marker::ContravariantLifetime::<'a>,
                         marker2: marker::NoCopy}
            }
        }
    }

    #[inline]
    fn last_mut(&mut self) -> Option<&mut T> {
        let len = self.len();
        if len == 0 { return None; }
        Some(&mut self[len - 1])
    }

    #[inline]
    fn head_mut(&mut self) -> Option<&mut T> {
        if self.len() == 0 { None } else { Some(&mut self[0]) }
    }

    #[inline]
    fn tail_mut(&mut self) -> &mut [T] {
        let len = self.len();
        self[mut 1..len]
    }

    #[inline]
    fn init_mut(&mut self) -> &mut [T] {
        let len = self.len();
        self[mut 0..len - 1]
    }

    #[inline]
    fn split_mut<'a>(&'a mut self, pred: |&T|: 'a -> bool) -> MutSplits<'a, T> {
        MutSplits { v: self, pred: pred, finished: false }
    }

    #[inline]
    fn splitn_mut<'a>(&'a mut self, n: uint, pred: |&T|: 'a -> bool) -> SplitsN<MutSplits<'a, T>> {
        SplitsN {
            iter: self.split_mut(pred),
            count: n,
            invert: false
        }
    }

    #[inline]
    fn rsplitn_mut<'a>(&'a mut self, n: uint, pred: |&T|: 'a -> bool) -> SplitsN<MutSplits<'a, T>> {
        SplitsN {
            iter: self.split_mut(pred),
            count: n,
            invert: true
        }
   }

    #[inline]
    fn chunks_mut(&mut self, chunk_size: uint) -> MutChunks<T> {
        assert!(chunk_size > 0);
        MutChunks { v: self, chunk_size: chunk_size }
    }

    fn swap(&mut self, a: uint, b: uint) {
        unsafe {
            // Can't take two mutable loans from one vector, so instead just cast
            // them to their raw pointers to do the swap
            let pa: *mut T = &mut self[a];
            let pb: *mut T = &mut self[b];
            ptr::swap(pa, pb);
        }
    }

    fn reverse(&mut self) {
        let mut i: uint = 0;
        let ln = self.len();
        while i < ln / 2 {
            // Unsafe swap to avoid the bounds check in safe swap.
            unsafe {
                let pa: *mut T = self.unsafe_mut(i);
                let pb: *mut T = self.unsafe_mut(ln - i - 1);
                ptr::swap(pa, pb);
            }
            i += 1;
        }
    }

    #[inline]
    unsafe fn unsafe_mut(&mut self, index: uint) -> &mut T {
        transmute((self.repr().data as *mut T).offset(index as int))
    }

    #[inline]
    fn as_mut_ptr(&mut self) -> *mut T {
        self.repr().data as *mut T
    }
}

impl<T> ops::Index<uint, T> for [T] {
    fn index(&self, &index: &uint) -> &T {
        assert!(index < self.len());

        unsafe { mem::transmute(self.repr().data.offset(index as int)) }
    }
}

impl<T> ops::IndexMut<uint, T> for [T] {
    fn index_mut(&mut self, &index: &uint) -> &mut T {
        assert!(index < self.len());

        unsafe { mem::transmute(self.repr().data.offset(index as int)) }
    }
}

impl<T> ops::Slice<uint, [T]> for [T] {
    #[inline]
    fn as_slice_<'a>(&'a self) -> &'a [T] {
        self
    }

    #[inline]
    fn slice_from_or_fail<'a>(&'a self, start: &uint) -> &'a [T] {
        self.slice_or_fail(start, &self.len())
    }

    #[inline]
    fn slice_to_or_fail<'a>(&'a self, end: &uint) -> &'a [T] {
        self.slice_or_fail(&0, end)
    }
    #[inline]
    fn slice_or_fail<'a>(&'a self, start: &uint, end: &uint) -> &'a [T] {
        assert!(*start <= *end);
        assert!(*end <= self.len());
        unsafe {
            transmute(RawSlice {
                    data: self.as_ptr().offset(*start as int),
                    len: (*end - *start)
                })
        }
    }
}

impl<T> ops::SliceMut<uint, [T]> for [T] {
    #[inline]
    fn as_mut_slice_<'a>(&'a mut self) -> &'a mut [T] {
        self
    }

    #[inline]
    fn slice_from_or_fail_mut<'a>(&'a mut self, start: &uint) -> &'a mut [T] {
        let len = &self.len();
        self.slice_or_fail_mut(start, len)
    }

    #[inline]
    fn slice_to_or_fail_mut<'a>(&'a mut self, end: &uint) -> &'a mut [T] {
        self.slice_or_fail_mut(&0, end)
    }
    #[inline]
    fn slice_or_fail_mut<'a>(&'a mut self, start: &uint, end: &uint) -> &'a mut [T] {
        assert!(*start <= *end);
        assert!(*end <= self.len());
        unsafe {
            transmute(RawSlice {
                    data: self.as_ptr().offset(*start as int),
                    len: (*end - *start)
                })
        }
    }
}

/// Extension methods for slices containing `PartialEq` elements.
#[unstable = "may merge with other traits"]
pub trait PartialEqSlicePrelude<T: PartialEq> for Sized? {
    /// Find the first index containing a matching value.
    fn position_elem(&self, t: &T) -> Option<uint>;

    /// Find the last index containing a matching value.
    fn rposition_elem(&self, t: &T) -> Option<uint>;

    /// Return true if the slice contains an element with the given value.
    fn contains(&self, x: &T) -> bool;

    /// Returns true if `needle` is a prefix of the slice.
    fn starts_with(&self, needle: &[T]) -> bool;

    /// Returns true if `needle` is a suffix of the slice.
    fn ends_with(&self, needle: &[T]) -> bool;
}

#[unstable = "trait is unstable"]
impl<T: PartialEq> PartialEqSlicePrelude<T> for [T] {
    #[inline]
    fn position_elem(&self, x: &T) -> Option<uint> {
        self.iter().position(|y| *x == *y)
    }

    #[inline]
    fn rposition_elem(&self, t: &T) -> Option<uint> {
        self.iter().rposition(|x| *x == *t)
    }

    #[inline]
    fn contains(&self, x: &T) -> bool {
        self.iter().any(|elt| *x == *elt)
    }

    #[inline]
    fn starts_with(&self, needle: &[T]) -> bool {
        let n = needle.len();
        self.len() >= n && needle == self[..n]
    }

    #[inline]
    fn ends_with(&self, needle: &[T]) -> bool {
        let (m, n) = (self.len(), needle.len());
        m >= n && needle == self[m-n..]
    }
}

/// Extension methods for slices containing `Ord` elements.
#[unstable = "may merge with other traits"]
pub trait OrdSlicePrelude<T: Ord> for Sized? {
    /// Binary search a sorted slice for a given element.
    ///
    /// If the value is found then `Found` is returned, containing the
    /// index of the matching element; if the value is not found then
    /// `NotFound` is returned, containing the index where a matching
    /// element could be inserted while maintaining sorted order.
    ///
    /// # Example
    ///
    /// Looks up a series of four elements. The first is found, with a
    /// uniquely determined position; the second and third are not
    /// found; the fourth could match any position in `[1,4]`.
    ///
    /// ```rust
    /// use std::slice::{Found, NotFound};
    /// let s = [0i, 1, 1, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55];
    /// let s = s.as_slice();
    ///
    /// assert_eq!(s.binary_search_elem(&13),  Found(9));
    /// assert_eq!(s.binary_search_elem(&4),   NotFound(7));
    /// assert_eq!(s.binary_search_elem(&100), NotFound(13));
    /// let r = s.binary_search_elem(&1);
    /// assert!(match r { Found(1...4) => true, _ => false, });
    /// ```
    #[unstable = "name likely to change"]
    fn binary_search_elem(&self, x: &T) -> BinarySearchResult;

    /// Mutates the slice to the next lexicographic permutation.
    ///
    /// Returns `true` if successful and `false` if the slice is at the
    /// last-ordered permutation.
    ///
    /// # Example
    ///
    /// ```rust
    /// let v: &mut [_] = &mut [0i, 1, 2];
    /// v.next_permutation();
    /// let b: &mut [_] = &mut [0i, 2, 1];
    /// assert!(v == b);
    /// v.next_permutation();
    /// let b: &mut [_] = &mut [1i, 0, 2];
    /// assert!(v == b);
    /// ```
    #[experimental]
    fn next_permutation(&mut self) -> bool;

    /// Mutates the slice to the previous lexicographic permutation.
    ///
    /// Returns `true` if successful and `false` if the slice is at the
    /// first-ordered permutation.
    ///
    /// # Example
    ///
    /// ```rust
    /// let v: &mut [_] = &mut [1i, 0, 2];
    /// v.prev_permutation();
    /// let b: &mut [_] = &mut [0i, 2, 1];
    /// assert!(v == b);
    /// v.prev_permutation();
    /// let b: &mut [_] = &mut [0i, 1, 2];
    /// assert!(v == b);
    /// ```
    #[experimental]
    fn prev_permutation(&mut self) -> bool;
}

#[unstable = "trait is unstable"]
impl<T: Ord> OrdSlicePrelude<T> for [T] {
    #[unstable]
    fn binary_search_elem(&self, x: &T) -> BinarySearchResult {
        self.binary_search(|p| p.cmp(x))
    }

    #[experimental]
    fn next_permutation(&mut self) -> bool {
        // These cases only have 1 permutation each, so we can't do anything.
        if self.len() < 2 { return false; }

        // Step 1: Identify the longest, rightmost weakly decreasing part of the vector
        let mut i = self.len() - 1;
        while i > 0 && self[i-1] >= self[i] {
            i -= 1;
        }

        // If that is the entire vector, this is the last-ordered permutation.
        if i == 0 {
            return false;
        }

        // Step 2: Find the rightmost element larger than the pivot (i-1)
        let mut j = self.len() - 1;
        while j >= i && self[j] <= self[i-1]  {
            j -= 1;
        }

        // Step 3: Swap that element with the pivot
        self.swap(j, i-1);

        // Step 4: Reverse the (previously) weakly decreasing part
        self[mut i..].reverse();

        true
    }

    #[experimental]
    fn prev_permutation(&mut self) -> bool {
        // These cases only have 1 permutation each, so we can't do anything.
        if self.len() < 2 { return false; }

        // Step 1: Identify the longest, rightmost weakly increasing part of the vector
        let mut i = self.len() - 1;
        while i > 0 && self[i-1] <= self[i] {
            i -= 1;
        }

        // If that is the entire vector, this is the first-ordered permutation.
        if i == 0 {
            return false;
        }

        // Step 2: Reverse the weakly increasing part
        self[mut i..].reverse();

        // Step 3: Find the rightmost element equal to or bigger than the pivot (i-1)
        let mut j = self.len() - 1;
        while j >= i && self[j-1] < self[i-1]  {
            j -= 1;
        }

        // Step 4: Swap that element with the pivot
        self.swap(i-1, j);

        true
    }
}

/// Extension methods for slices on Clone elements
#[unstable = "may merge with other traits"]
pub trait CloneSlicePrelude<T> for Sized? {
    /// Copies as many elements from `src` as it can into `self` (the
    /// shorter of `self.len()` and `src.len()`). Returns the number
    /// of elements copied.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::slice::CloneSlicePrelude;
    ///
    /// let mut dst = [0i, 0, 0];
    /// let src = [1i, 2];
    ///
    /// assert!(dst.clone_from_slice(&src) == 2);
    /// assert!(dst == [1, 2, 0]);
    ///
    /// let src2 = [3i, 4, 5, 6];
    /// assert!(dst.clone_from_slice(&src2) == 3);
    /// assert!(dst == [3i, 4, 5]);
    /// ```
    fn clone_from_slice(&mut self, &[T]) -> uint;
}

#[unstable = "trait is unstable"]
impl<T: Clone> CloneSlicePrelude<T> for [T] {
    #[inline]
    fn clone_from_slice(&mut self, src: &[T]) -> uint {
        let min = cmp::min(self.len(), src.len());
        let dst = self.slice_to_mut(min);
        let src = src.slice_to(min);
        for i in range(0, min) {
            dst[i].clone_from(&src[i]);
        }
        min
    }
}




//
// Common traits
//

/// Data that is viewable as a slice.
#[unstable = "may merge with other traits"]
pub trait AsSlice<T> for Sized? {
    /// Work with `self` as a slice.
    fn as_slice<'a>(&'a self) -> &'a [T];
}

#[unstable = "trait is unstable"]
impl<T> AsSlice<T> for [T] {
    #[inline(always)]
    fn as_slice<'a>(&'a self) -> &'a [T] { self }
}

impl<'a, T, Sized? U: AsSlice<T>> AsSlice<T> for &'a U {
    #[inline(always)]
    fn as_slice<'a>(&'a self) -> &'a [T] { AsSlice::as_slice(*self) }
}

impl<'a, T, Sized? U: AsSlice<T>> AsSlice<T> for &'a mut U {
    #[inline(always)]
    fn as_slice<'a>(&'a self) -> &'a [T] { AsSlice::as_slice(*self) }
}

#[unstable = "waiting for DST"]
impl<'a, T> Default for &'a [T] {
    fn default() -> &'a [T] { &[] }
}

//
// Iterators
//

// The shared definition of the `Item` and `MutItems` iterators
macro_rules! iterator {
    (struct $name:ident -> $ptr:ty, $elem:ty) => {
        #[experimental = "needs review"]
        impl<'a, T> Iterator<$elem> for $name<'a, T> {
            #[inline]
            fn next(&mut self) -> Option<$elem> {
                // could be implemented with slices, but this avoids bounds checks
                unsafe {
                    if self.ptr == self.end {
                        None
                    } else {
                        if mem::size_of::<T>() == 0 {
                            // purposefully don't use 'ptr.offset' because for
                            // vectors with 0-size elements this would return the
                            // same pointer.
                            self.ptr = transmute(self.ptr as uint + 1);

                            // Use a non-null pointer value
                            Some(transmute(1u))
                        } else {
                            let old = self.ptr;
                            self.ptr = self.ptr.offset(1);

                            Some(transmute(old))
                        }
                    }
                }
            }

            #[inline]
            fn size_hint(&self) -> (uint, Option<uint>) {
                let diff = (self.end as uint) - (self.ptr as uint);
                let size = mem::size_of::<T>();
                let exact = diff / (if size == 0 {1} else {size});
                (exact, Some(exact))
            }
        }

        #[experimental = "needs review"]
        impl<'a, T> DoubleEndedIterator<$elem> for $name<'a, T> {
            #[inline]
            fn next_back(&mut self) -> Option<$elem> {
                // could be implemented with slices, but this avoids bounds checks
                unsafe {
                    if self.end == self.ptr {
                        None
                    } else {
                        if mem::size_of::<T>() == 0 {
                            // See above for why 'ptr.offset' isn't used
                            self.end = transmute(self.end as uint - 1);

                            // Use a non-null pointer value
                            Some(transmute(1u))
                        } else {
                            self.end = self.end.offset(-1);

                            Some(transmute(self.end))
                        }
                    }
                }
            }
        }
    }
}

macro_rules! make_slice {
    ($t: ty -> $result: ty: $start: expr, $end: expr) => {{
        let diff = $end as uint - $start as uint;
        let len = if mem::size_of::<T>() == 0 {
            diff
        } else {
            diff / mem::size_of::<$t>()
        };
        unsafe {
            transmute::<_, $result>(RawSlice { data: $start as *const T, len: len })
        }
    }}
}


/// Immutable slice iterator
#[experimental = "needs review"]
pub struct Items<'a, T: 'a> {
    ptr: *const T,
    end: *const T,
    marker: marker::ContravariantLifetime<'a>
}

#[experimental]
impl<'a, T> ops::Slice<uint, [T]> for Items<'a, T> {
    fn as_slice_(&self) -> &[T] {
        self.as_slice()
    }
    fn slice_from_or_fail<'b>(&'b self, from: &uint) -> &'b [T] {
        use ops::Slice;
        self.as_slice().slice_from_or_fail(from)
    }
    fn slice_to_or_fail<'b>(&'b self, to: &uint) -> &'b [T] {
        use ops::Slice;
        self.as_slice().slice_to_or_fail(to)
    }
    fn slice_or_fail<'b>(&'b self, from: &uint, to: &uint) -> &'b [T] {
        use ops::Slice;
        self.as_slice().slice_or_fail(from, to)
    }
}

impl<'a, T> Items<'a, T> {
    /// View the underlying data as a subslice of the original data.
    ///
    /// This has the same lifetime as the original slice, and so the
    /// iterator can continue to be used while this exists.
    #[experimental]
    pub fn as_slice(&self) -> &'a [T] {
        make_slice!(T -> &'a [T]: self.ptr, self.end)
    }
}

iterator!{struct Items -> *const T, &'a T}

#[experimental = "needs review"]
impl<'a, T> ExactSize<&'a T> for Items<'a, T> {}

#[experimental = "needs review"]
impl<'a, T> Clone for Items<'a, T> {
    fn clone(&self) -> Items<'a, T> { *self }
}

#[experimental = "needs review"]
impl<'a, T> RandomAccessIterator<&'a T> for Items<'a, T> {
    #[inline]
    fn indexable(&self) -> uint {
        let (exact, _) = self.size_hint();
        exact
    }

    #[inline]
    fn idx(&mut self, index: uint) -> Option<&'a T> {
        unsafe {
            if index < self.indexable() {
                if mem::size_of::<T>() == 0 {
                    // Use a non-null pointer value
                    Some(transmute(1u))
                } else {
                    Some(transmute(self.ptr.offset(index as int)))
                }
            } else {
                None
            }
        }
    }
}

/// Mutable slice iterator.
#[experimental = "needs review"]
pub struct MutItems<'a, T: 'a> {
    ptr: *mut T,
    end: *mut T,
    marker: marker::ContravariantLifetime<'a>,
    marker2: marker::NoCopy
}

#[experimental]
impl<'a, T> ops::Slice<uint, [T]> for MutItems<'a, T> {
    fn as_slice_<'b>(&'b self) -> &'b [T] {
        make_slice!(T -> &'b [T]: self.ptr, self.end)
    }
    fn slice_from_or_fail<'b>(&'b self, from: &uint) -> &'b [T] {
        use ops::Slice;
        self.as_slice_().slice_from_or_fail(from)
    }
    fn slice_to_or_fail<'b>(&'b self, to: &uint) -> &'b [T] {
        use ops::Slice;
        self.as_slice_().slice_to_or_fail(to)
    }
    fn slice_or_fail<'b>(&'b self, from: &uint, to: &uint) -> &'b [T] {
        use ops::Slice;
        self.as_slice_().slice_or_fail(from, to)
    }
}

#[experimental]
impl<'a, T> ops::SliceMut<uint, [T]> for MutItems<'a, T> {
    fn as_mut_slice_<'b>(&'b mut self) -> &'b mut [T] {
        make_slice!(T -> &'b mut [T]: self.ptr, self.end)
    }
    fn slice_from_or_fail_mut<'b>(&'b mut self, from: &uint) -> &'b mut [T] {
        use ops::SliceMut;
        self.as_mut_slice_().slice_from_or_fail_mut(from)
    }
    fn slice_to_or_fail_mut<'b>(&'b mut self, to: &uint) -> &'b mut [T] {
        use ops::SliceMut;
        self.as_mut_slice_().slice_to_or_fail_mut(to)
    }
    fn slice_or_fail_mut<'b>(&'b mut self, from: &uint, to: &uint) -> &'b mut [T] {
        use ops::SliceMut;
        self.as_mut_slice_().slice_or_fail_mut(from, to)
    }
}

impl<'a, T> MutItems<'a, T> {
    /// View the underlying data as a subslice of the original data.
    ///
    /// To avoid creating `&mut` references that alias, this is forced
    /// to consume the iterator. Consider using the `Slice` and
    /// `SliceMut` implementations for obtaining slices with more
    /// restricted lifetimes that do not consume the iterator.
    #[experimental]
    pub fn into_slice(self) -> &'a mut [T] {
        make_slice!(T -> &'a mut [T]: self.ptr, self.end)
    }
}

iterator!{struct MutItems -> *mut T, &'a mut T}

#[experimental = "needs review"]
impl<'a, T> ExactSize<&'a mut T> for MutItems<'a, T> {}

/// An abstraction over the splitting iterators, so that splitn, splitn_mut etc
/// can be implemented once.
trait SplitsIter<E>: DoubleEndedIterator<E> {
    /// Mark the underlying iterator as complete, extracting the remaining
    /// portion of the slice.
    fn finish(&mut self) -> Option<E>;
}

/// An iterator over subslices separated by elements that match a predicate
/// function.
#[experimental = "needs review"]
pub struct Splits<'a, T:'a> {
    v: &'a [T],
    pred: |t: &T|: 'a -> bool,
    finished: bool
}

#[experimental = "needs review"]
impl<'a, T> Iterator<&'a [T]> for Splits<'a, T> {
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }

        match self.v.iter().position(|x| (self.pred)(x)) {
            None => self.finish(),
            Some(idx) => {
                let ret = Some(self.v[..idx]);
                self.v = self.v[idx + 1..];
                ret
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        if self.finished {
            (0, Some(0))
        } else {
            (1, Some(self.v.len() + 1))
        }
    }
}

#[experimental = "needs review"]
impl<'a, T> DoubleEndedIterator<&'a [T]> for Splits<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.finished { return None; }

        match self.v.iter().rposition(|x| (self.pred)(x)) {
            None => self.finish(),
            Some(idx) => {
                let ret = Some(self.v[idx + 1..]);
                self.v = self.v[..idx];
                ret
            }
        }
    }
}

impl<'a, T> SplitsIter<&'a [T]> for Splits<'a, T> {
    #[inline]
    fn finish(&mut self) -> Option<&'a [T]> {
        if self.finished { None } else { self.finished = true; Some(self.v) }
    }
}

/// An iterator over the subslices of the vector which are separated
/// by elements that match `pred`.
#[experimental = "needs review"]
pub struct MutSplits<'a, T:'a> {
    v: &'a mut [T],
    pred: |t: &T|: 'a -> bool,
    finished: bool
}

impl<'a, T> SplitsIter<&'a mut [T]> for MutSplits<'a, T> {
    #[inline]
    fn finish(&mut self) -> Option<&'a mut [T]> {
        if self.finished {
            None
        } else {
            self.finished = true;
            Some(mem::replace(&mut self.v, &mut []))
        }
    }
}

#[experimental = "needs review"]
impl<'a, T> Iterator<&'a mut [T]> for MutSplits<'a, T> {
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }

        let idx_opt = { // work around borrowck limitations
            let pred = &mut self.pred;
            self.v.iter().position(|x| (*pred)(x))
        };
        match idx_opt {
            None => self.finish(),
            Some(idx) => {
                let tmp = mem::replace(&mut self.v, &mut []);
                let (head, tail) = tmp.split_at_mut(idx);
                self.v = tail[mut 1..];
                Some(head)
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        if self.finished {
            (0, Some(0))
        } else {
            // if the predicate doesn't match anything, we yield one slice
            // if it matches every element, we yield len+1 empty slices.
            (1, Some(self.v.len() + 1))
        }
    }
}

#[experimental = "needs review"]
impl<'a, T> DoubleEndedIterator<&'a mut [T]> for MutSplits<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.finished { return None; }

        let idx_opt = { // work around borrowck limitations
            let pred = &mut self.pred;
            self.v.iter().rposition(|x| (*pred)(x))
        };
        match idx_opt {
            None => self.finish(),
            Some(idx) => {
                let tmp = mem::replace(&mut self.v, &mut []);
                let (head, tail) = tmp.split_at_mut(idx);
                self.v = head;
                Some(tail[mut 1..])
            }
        }
    }
}

/// An iterator over subslices separated by elements that match a predicate
/// function, splitting at most a fixed number of times.
#[experimental = "needs review"]
pub struct SplitsN<I> {
    iter: I,
    count: uint,
    invert: bool
}

#[experimental = "needs review"]
impl<E, I: SplitsIter<E>> Iterator<E> for SplitsN<I> {
    #[inline]
    fn next(&mut self) -> Option<E> {
        if self.count == 0 {
            self.iter.finish()
        } else {
            self.count -= 1;
            if self.invert { self.iter.next_back() } else { self.iter.next() }
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        let (lower, upper_opt) = self.iter.size_hint();
        (lower, upper_opt.map(|upper| cmp::min(self.count + 1, upper)))
    }
}

/// An iterator over overlapping subslices of length `size`.
#[deriving(Clone)]
#[experimental = "needs review"]
pub struct Windows<'a, T:'a> {
    v: &'a [T],
    size: uint
}

impl<'a, T> Iterator<&'a [T]> for Windows<'a, T> {
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        if self.size > self.v.len() {
            None
        } else {
            let ret = Some(self.v[..self.size]);
            self.v = self.v[1..];
            ret
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        if self.size > self.v.len() {
            (0, Some(0))
        } else {
            let x = self.v.len() - self.size;
            (x.saturating_add(1), x.checked_add(1u))
        }
    }
}

/// An iterator over a slice in (non-overlapping) chunks (`size` elements at a
/// time).
///
/// When the slice len is not evenly divided by the chunk size, the last slice
/// of the iteration will be the remainder.
#[deriving(Clone)]
#[experimental = "needs review"]
pub struct Chunks<'a, T:'a> {
    v: &'a [T],
    size: uint
}

#[experimental = "needs review"]
impl<'a, T> Iterator<&'a [T]> for Chunks<'a, T> {
    #[inline]
    fn next(&mut self) -> Option<&'a [T]> {
        if self.v.len() == 0 {
            None
        } else {
            let chunksz = cmp::min(self.v.len(), self.size);
            let (fst, snd) = self.v.split_at(chunksz);
            self.v = snd;
            Some(fst)
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        if self.v.len() == 0 {
            (0, Some(0))
        } else {
            let n = self.v.len() / self.size;
            let rem = self.v.len() % self.size;
            let n = if rem > 0 { n+1 } else { n };
            (n, Some(n))
        }
    }
}

#[experimental = "needs review"]
impl<'a, T> DoubleEndedIterator<&'a [T]> for Chunks<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [T]> {
        if self.v.len() == 0 {
            None
        } else {
            let remainder = self.v.len() % self.size;
            let chunksz = if remainder != 0 { remainder } else { self.size };
            let (fst, snd) = self.v.split_at(self.v.len() - chunksz);
            self.v = fst;
            Some(snd)
        }
    }
}

#[experimental = "needs review"]
impl<'a, T> RandomAccessIterator<&'a [T]> for Chunks<'a, T> {
    #[inline]
    fn indexable(&self) -> uint {
        self.v.len()/self.size + if self.v.len() % self.size != 0 { 1 } else { 0 }
    }

    #[inline]
    fn idx(&mut self, index: uint) -> Option<&'a [T]> {
        if index < self.indexable() {
            let lo = index * self.size;
            let mut hi = lo + self.size;
            if hi < lo || hi > self.v.len() { hi = self.v.len(); }

            Some(self.v[lo..hi])
        } else {
            None
        }
    }
}

/// An iterator over a slice in (non-overlapping) mutable chunks (`size`
/// elements at a time). When the slice len is not evenly divided by the chunk
/// size, the last slice of the iteration will be the remainder.
#[experimental = "needs review"]
pub struct MutChunks<'a, T:'a> {
    v: &'a mut [T],
    chunk_size: uint
}

#[experimental = "needs review"]
impl<'a, T> Iterator<&'a mut [T]> for MutChunks<'a, T> {
    #[inline]
    fn next(&mut self) -> Option<&'a mut [T]> {
        if self.v.len() == 0 {
            None
        } else {
            let sz = cmp::min(self.v.len(), self.chunk_size);
            let tmp = mem::replace(&mut self.v, &mut []);
            let (head, tail) = tmp.split_at_mut(sz);
            self.v = tail;
            Some(head)
        }
    }

    #[inline]
    fn size_hint(&self) -> (uint, Option<uint>) {
        if self.v.len() == 0 {
            (0, Some(0))
        } else {
            let n = self.v.len() / self.chunk_size;
            let rem = self.v.len() % self.chunk_size;
            let n = if rem > 0 { n + 1 } else { n };
            (n, Some(n))
        }
    }
}

#[experimental = "needs review"]
impl<'a, T> DoubleEndedIterator<&'a mut [T]> for MutChunks<'a, T> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [T]> {
        if self.v.len() == 0 {
            None
        } else {
            let remainder = self.v.len() % self.chunk_size;
            let sz = if remainder != 0 { remainder } else { self.chunk_size };
            let tmp = mem::replace(&mut self.v, &mut []);
            let tmp_len = tmp.len();
            let (head, tail) = tmp.split_at_mut(tmp_len - sz);
            self.v = head;
            Some(tail)
        }
    }
}



/// The result of calling `binary_search`.
///
/// `Found` means the search succeeded, and the contained value is the
/// index of the matching element. `NotFound` means the search
/// succeeded, and the contained value is an index where a matching
/// value could be inserted while maintaining sort order.
#[deriving(PartialEq, Show)]
#[experimental = "needs review"]
pub enum BinarySearchResult {
    /// The index of the found value.
    Found(uint),
    /// The index where the value should have been found.
    NotFound(uint)
}

#[experimental = "needs review"]
impl BinarySearchResult {
    /// Converts a `Found` to `Some`, `NotFound` to `None`.
    /// Similar to `Result::ok`.
    pub fn found(&self) -> Option<uint> {
        match *self {
            Found(i) => Some(i),
            NotFound(_) => None
        }
    }

    /// Convert a `Found` to `None`, `NotFound` to `Some`.
    /// Similar to `Result::err`.
    pub fn not_found(&self) -> Option<uint> {
        match *self {
            Found(_) => None,
            NotFound(i) => Some(i)
        }
    }
}



//
// Free functions
//

/**
 * Converts a pointer to A into a slice of length 1 (without copying).
 */
#[unstable = "waiting for DST"]
pub fn ref_slice<'a, A>(s: &'a A) -> &'a [A] {
    unsafe {
        transmute(RawSlice { data: s, len: 1 })
    }
}

/**
 * Converts a pointer to A into a slice of length 1 (without copying).
 */
#[unstable = "waiting for DST"]
pub fn mut_ref_slice<'a, A>(s: &'a mut A) -> &'a mut [A] {
    unsafe {
        let ptr: *const A = transmute(s);
        transmute(RawSlice { data: ptr, len: 1 })
    }
}

/// Forms a slice from a pointer and a length.
///
/// The pointer given is actually a reference to the base of the slice. This
/// reference is used to give a concrete lifetime to tie the returned slice to.
/// Typically this should indicate that the slice is valid for as long as the
/// pointer itself is valid.
///
/// The `len` argument is the number of **elements**, not the number of bytes.
///
/// This function is unsafe as there is no guarantee that the given pointer is
/// valid for `len` elements, nor whether the lifetime provided is a suitable
/// lifetime for the returned slice.
///
/// # Example
///
/// ```rust
/// use std::slice;
///
/// // manifest a slice out of thin air!
/// let ptr = 0x1234 as *const uint;
/// let amt = 10;
/// unsafe {
///     let slice = slice::from_raw_buf(&ptr, amt);
/// }
/// ```
#[inline]
#[unstable = "just renamed from `mod raw`"]
pub unsafe fn from_raw_buf<'a, T>(p: &'a *const T, len: uint) -> &'a [T] {
    transmute(RawSlice { data: *p, len: len })
}

/// Performs the same functionality as `from_raw_buf`, except that a mutable
/// slice is returned.
///
/// This function is unsafe for the same reasons as `from_raw_buf`, as well as
/// not being able to provide a non-aliasing guarantee of the returned mutable
/// slice.
#[inline]
#[unstable = "just renamed from `mod raw`"]
pub unsafe fn from_raw_mut_buf<'a, T>(p: &'a *mut T, len: uint) -> &'a mut [T] {
    transmute(RawSlice { data: *p as *const T, len: len })
}

//
// Submodules
//

/// Unsafe operations
#[deprecated]
pub mod raw {
    use mem::transmute;
    use ptr::RawPtr;
    use raw::Slice;
    use option::{None, Option, Some};

    /**
     * Form a slice from a pointer and length (as a number of units,
     * not bytes).
     */
    #[inline]
    #[deprecated = "renamed to slice::from_raw_buf"]
    pub unsafe fn buf_as_slice<T,U>(p: *const T, len: uint, f: |v: &[T]| -> U)
                               -> U {
        f(transmute(Slice {
            data: p,
            len: len
        }))
    }

    /**
     * Form a slice from a pointer and length (as a number of units,
     * not bytes).
     */
    #[inline]
    #[deprecated = "renamed to slice::from_raw_mut_buf"]
    pub unsafe fn mut_buf_as_slice<T,
                                   U>(
                                   p: *mut T,
                                   len: uint,
                                   f: |v: &mut [T]| -> U)
                                   -> U {
        f(transmute(Slice {
            data: p as *const T,
            len: len
        }))
    }

    /**
     * Returns a pointer to first element in slice and adjusts
     * slice so it no longer contains that element. Returns None
     * if the slice is empty. O(1).
     */
     #[inline]
    #[deprecated = "inspect `Slice::{data, len}` manually (increment data by 1)"]
    pub unsafe fn shift_ptr<T>(slice: &mut Slice<T>) -> Option<*const T> {
        if slice.len == 0 { return None; }
        let head: *const T = slice.data;
        slice.data = slice.data.offset(1);
        slice.len -= 1;
        Some(head)
    }

    /**
     * Returns a pointer to last element in slice and adjusts
     * slice so it no longer contains that element. Returns None
     * if the slice is empty. O(1).
     */
    #[inline]
    #[deprecated = "inspect `Slice::{data, len}` manually (decrement len by 1)"]
    pub unsafe fn pop_ptr<T>(slice: &mut Slice<T>) -> Option<*const T> {
        if slice.len == 0 { return None; }
        let tail: *const T = slice.data.offset((slice.len - 1) as int);
        slice.len -= 1;
        Some(tail)
    }
}

/// Operations on `[u8]`.
#[experimental = "needs review"]
pub mod bytes {
    use kinds::Sized;
    use ptr;
    use slice::SlicePrelude;

    /// A trait for operations on mutable `[u8]`s.
    pub trait MutableByteVector for Sized? {
        /// Sets all bytes of the receiver to the given value.
        fn set_memory(&mut self, value: u8);
    }

    impl MutableByteVector for [u8] {
        #[inline]
        #[allow(experimental)]
        fn set_memory(&mut self, value: u8) {
            unsafe { ptr::set_memory(self.as_mut_ptr(), value, self.len()) };
        }
    }

    /// Copies data from `src` to `dst`
    ///
    /// `src` and `dst` must not overlap. Panics if the length of `dst`
    /// is less than the length of `src`.
    #[inline]
    pub fn copy_memory(dst: &mut [u8], src: &[u8]) {
        let len_src = src.len();
        assert!(dst.len() >= len_src);
        unsafe {
            ptr::copy_nonoverlapping_memory(dst.as_mut_ptr(),
                                            src.as_ptr(),
                                            len_src);
        }
    }
}



//
// Boilerplate traits
//

#[unstable = "waiting for DST"]
impl<T: PartialEq> PartialEq for [T] {
    fn eq(&self, other: &[T]) -> bool {
        self.len() == other.len() &&
            order::eq(self.iter(), other.iter())
    }
    fn ne(&self, other: &[T]) -> bool {
        self.len() != other.len() ||
            order::ne(self.iter(), other.iter())
    }
}

#[unstable = "waiting for DST"]
impl<T: Eq> Eq for [T] {}

#[unstable = "waiting for DST"]
impl<T: PartialEq, Sized? V: AsSlice<T>> Equiv<V> for [T] {
    #[inline]
    fn equiv(&self, other: &V) -> bool { self.as_slice() == other.as_slice() }
}

#[unstable = "waiting for DST"]
impl<'a,T:PartialEq, Sized? V: AsSlice<T>> Equiv<V> for &'a mut [T] {
    #[inline]
    fn equiv(&self, other: &V) -> bool { self.as_slice() == other.as_slice() }
}

#[unstable = "waiting for DST"]
impl<T: Ord> Ord for [T] {
    fn cmp(&self, other: &[T]) -> Ordering {
        order::cmp(self.iter(), other.iter())
    }
}

#[unstable = "waiting for DST"]
impl<T: PartialOrd> PartialOrd for [T] {
    #[inline]
    fn partial_cmp(&self, other: &[T]) -> Option<Ordering> {
        order::partial_cmp(self.iter(), other.iter())
    }
    #[inline]
    fn lt(&self, other: &[T]) -> bool {
        order::lt(self.iter(), other.iter())
    }
    #[inline]
    fn le(&self, other: &[T]) -> bool {
        order::le(self.iter(), other.iter())
    }
    #[inline]
    fn ge(&self, other: &[T]) -> bool {
        order::ge(self.iter(), other.iter())
    }
    #[inline]
    fn gt(&self, other: &[T]) -> bool {
        order::gt(self.iter(), other.iter())
    }
}

/// Extension methods for immutable slices containing integers.
#[experimental]
pub trait ImmutableIntSlice<U, S> for Sized? {
    /// Converts the slice to an immutable slice of unsigned integers with the same width.
    fn as_unsigned<'a>(&'a self) -> &'a [U];
    /// Converts the slice to an immutable slice of signed integers with the same width.
    fn as_signed<'a>(&'a self) -> &'a [S];
}

/// Extension methods for mutable slices containing integers.
#[experimental]
pub trait MutableIntSlice<U, S> for Sized?: ImmutableIntSlice<U, S> {
    /// Converts the slice to a mutable slice of unsigned integers with the same width.
    fn as_unsigned_mut<'a>(&'a mut self) -> &'a mut [U];
    /// Converts the slice to a mutable slice of signed integers with the same width.
    fn as_signed_mut<'a>(&'a mut self) -> &'a mut [S];
}

macro_rules! impl_immut_int_slice {
    ($u:ty, $s:ty, $t:ty) => {
        #[experimental]
        impl ImmutableIntSlice<$u, $s> for [$t] {
            #[inline]
            fn as_unsigned(&self) -> &[$u] { unsafe { transmute(self) } }
            #[inline]
            fn as_signed(&self) -> &[$s] { unsafe { transmute(self) } }
        }
    }
}
macro_rules! impl_mut_int_slice {
    ($u:ty, $s:ty, $t:ty) => {
        #[experimental]
        impl MutableIntSlice<$u, $s> for [$t] {
            #[inline]
            fn as_unsigned_mut(&mut self) -> &mut [$u] { unsafe { transmute(self) } }
            #[inline]
            fn as_signed_mut(&mut self) -> &mut [$s] { unsafe { transmute(self) } }
        }
    }
}

macro_rules! impl_int_slice {
    ($u:ty, $s:ty) => {
        impl_immut_int_slice!($u, $s, $u)
        impl_immut_int_slice!($u, $s, $s)
        impl_mut_int_slice!($u, $s, $u)
        impl_mut_int_slice!($u, $s, $s)
    }
}

impl_int_slice!(u8,   i8)
impl_int_slice!(u16,  i16)
impl_int_slice!(u32,  i32)
impl_int_slice!(u64,  i64)
impl_int_slice!(uint, int)
