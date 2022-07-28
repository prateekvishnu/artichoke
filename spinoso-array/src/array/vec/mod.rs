//! Ruby `Array` based on [`Vec`].

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp;
use core::slice::{Iter, IterMut};

#[doc(inline)]
pub use raw_parts::RawParts;

mod convert;
mod eq;
mod impls;
mod iter;

/// A contiguous growable array type based on [`Vec<T>`](Vec).
///
/// `Array` implements indexing and mutating APIs that make an ideal backend for
/// the [Ruby `Array` core class][ruby-array]. In practice, this results in less
/// generic, more single-use APIs. For example, instead of [`Vec::drain`],
/// `Array` implements [`shift`], [`shift_n`], [`pop`], and [`pop_n`].
///
/// Similarly, slicing APIs are more specialized, such as [`first_n`] and
/// [`last_n`]. Slicing APIs do not return [`Option`], instead preferring to
/// return an empty slice.
///
/// # Examples
///
/// ```
/// # use spinoso_array::Array;
/// let mut ary = Array::new();
/// ary.push(1);
/// ary.push(2);
///
/// assert_eq!(ary.len(), 2);
/// assert_eq!(ary[0], 1);
///
/// assert_eq!(ary.pop(), Some(2));
/// assert_eq!(ary.len(), 1);
///
/// ary[0] = 7;
/// assert_eq!(ary[0], 7);
///
/// ary.extend([1, 2, 3].iter().copied());
///
/// for x in &ary {
///     println!("{}", x);
/// }
/// assert_eq!(ary, &[7, 1, 2, 3]);
/// ```
///
/// [ruby-array]: https://ruby-doc.org/core-2.6.3/Array.html
/// [`shift`]: Array::shift
/// [`shift_n`]: Array::shift_n
/// [`drop_n`]: Array::drop_n
/// [`pop`]: Array::pop
/// [`pop_n`]: Array::pop_n
/// [`first_n`]: Array::first_n
/// [`last_n`]: Array::last_n
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array<T>(Vec<T>);

impl<T> Default for Array<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Array<T> {
    /// Construct a new, empty `Array<T>`.
    ///
    /// The vector will not allocate until elements are pushed into it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary: Array<i32> = Array::new();
    /// assert!(ary.is_empty());
    /// assert_eq!(ary.capacity(), 0);
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Construct a new, empty `Array<T>` with the specified capacity.
    ///
    /// The vector will be able to hold exactly `capacity` elements without
    /// reallocating. If `capacity` is 0, the vector will not allocate.
    ///
    /// It is important to note that although the returned vector has the
    /// _capacity_ specified, the vector will have a zero _length_.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary: Array<i32> = Array::with_capacity(10);
    /// assert_eq!(ary.len(), 0);
    /// assert_eq!(ary.capacity(), 10);
    ///
    /// // These are pushes all done without reallocating...
    /// for i in 0..10 {
    ///     ary.push(i);
    /// }
    ///
    /// // ...but this may make the vector reallocate
    /// ary.push(11);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Construct a new two-element `Array` from the given arguments.
    ///
    /// The vector is constructed with `capacity` of 2.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::assoc(0, 100);
    /// assert_eq!(ary.capacity(), 2);
    /// assert_eq!(ary.len(), 2);
    /// assert_eq!(ary[0], 0);
    /// assert_eq!(ary[1], 100);
    /// ```
    #[inline]
    #[must_use]
    pub fn assoc(first: T, second: T) -> Self {
        Self(vec![first, second])
    }

    /// Returns an iterator over the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// let mut iterator = ary.iter();
    ///
    /// assert_eq!(iterator.next(), Some(&1));
    /// assert_eq!(iterator.next(), Some(&2));
    /// assert_eq!(iterator.next(), Some(&4));
    /// assert_eq!(iterator.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        self.into_iter()
    }

    /// Returns an iterator that allows modifying each value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// for elem in ary.iter_mut() {
    ///     *elem += 2;
    /// }
    ///
    /// assert_eq!(ary, &[3, 4, 6]);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.into_iter()
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&ary[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// let four_index = ary.as_slice().binary_search(&4);
    /// assert_eq!(four_index, Ok(2));
    /// ```
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    /// Extracts a mutable slice containing the entire vector.
    ///
    /// Equivalent to `&mut ary[..]`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[2, 1, 4]);
    /// ary.as_mut_slice().sort();
    /// assert_eq!(ary, &[1, 2, 4]);
    /// ```
    #[inline]
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.0.as_mut_slice()
    }

    /// Returns a raw pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage. Modifying
    /// the vector may cause its buffer to be reallocated, which would also make
    /// any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer
    /// (non-transitively) points to is never written to (except inside an
    /// `UnsafeCell`) using this pointer or any pointer derived from it. If you
    /// need to mutate the contents of the slice, use
    /// [`as_mut_ptr`](Self::as_mut_ptr).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// let ary_ptr = ary.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..ary.len() {
    ///         assert_eq!(*ary_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }

    /// Returns an unsafe mutable pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    /// Modifying the vector may cause its buffer to be reallocated, which would
    /// also make any pointers to it invalid.
    ///
    /// # Examples
    ///
    /// This method is primarily used when mutating a `Array` via a raw pointer
    /// passed over FFI.
    ///
    /// See the [`ARY_PTR`] macro in mruby.
    ///
    /// [`ARY_PTR`]: https://github.com/artichoke/mruby/blob/d66440864d08f1c3ac5820d45f11df031b7d43c6/include/mruby/array.h#L52
    #[inline]
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.0.as_mut_ptr()
    }

    /// Set the vector's length without dropping or moving out elements
    ///
    /// This method is unsafe because it changes the notion of the number of
    /// "valid" elements in the vector. Use with care.
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to capacity().
    /// - The elements at `old_len..new_len` must be initialized.
    ///
    /// # Examples
    ///
    /// This method is primarily used when mutating a `Array` via a raw pointer
    /// passed over FFI.
    ///
    /// See the [`ARY_PTR`] macro in mruby.
    ///
    /// [`ARY_PTR`]: https://github.com/artichoke/mruby/blob/d66440864d08f1c3ac5820d45f11df031b7d43c6/include/mruby/array.h#L52
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.0.set_len(new_len);
    }

    /// Creates an `Array<T>` directly from the raw components of another array.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// - `ptr` needs to have been previously allocated via `Array<T>` (at
    ///   least, it's highly likely to be incorrect if it wasn't).
    /// - `T` needs to have the same size and alignment as what `ptr` was
    ///   allocated with. (`T` having a less strict alignment is not sufficient,
    ///   the alignment really needs to be equal to satisfy the `dealloc`
    ///   requirement that memory must be allocated and deallocated with the
    ///   same layout.)
    /// - `length` needs to be less than or equal to `capacity`.
    /// - `capacity` needs to be the `capacity` that the pointer was allocated
    ///   with.
    ///
    /// Violating these may cause problems like corrupting the allocator's
    /// internal data structures.
    ///
    /// The ownership of `ptr` is effectively transferred to the `Array<T>`
    /// which may then deallocate, reallocate or change the contents of memory
    /// pointed to by the pointer at will. Ensure that nothing else uses the
    /// pointer after calling this function.
    #[must_use]
    pub unsafe fn from_raw_parts(raw_parts: RawParts<T>) -> Self {
        let vec = RawParts::into_vec(raw_parts);
        Self(vec)
    }

    /// Decomposes an `Array<T>` into its raw components.
    ///
    /// Returns the raw pointer to the underlying data, the length of the array
    /// (in elements), and the allocated capacity of the data (in elements).
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the `Array`. The only way to do this is to convert
    /// the raw pointer, length, and capacity back into a `Array` with the
    /// [`from_raw_parts`] function, allowing the destructor to perform the
    /// cleanup.
    ///
    /// [`from_raw_parts`]: Array::from_raw_parts
    #[must_use]
    pub fn into_raw_parts(self) -> RawParts<T> {
        RawParts::from_vec(self.0)
    }

    /// Consume the array and return the inner [`Vec<T>`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// let vec: Vec<i32> = ary.into_inner();
    /// ```
    ///
    /// [`Vec<T>`]: alloc::vec::Vec
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> Vec<T> {
        self.0
    }

    /// Consume the array and return its elements as a [`Vec<T>`].
    ///
    /// For `Array`, this is a cheap operation that unwraps the inner `Vec` and
    /// is an alias for [`into_inner`](Self::into_inner).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// let vec: Vec<i32> = ary.into_vec();
    /// ```
    ///
    /// [`Vec<T>`]: alloc::vec::Vec
    #[inline]
    #[must_use]
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }

    /// Converts the vector into [`Box<[T]>`](Box).
    ///
    /// This will drop any excess capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// let slice: Box<[i32]> = ary.into_boxed_slice();
    /// ```
    #[inline]
    #[must_use]
    pub fn into_boxed_slice(self) -> Box<[T]> {
        self.0.into_boxed_slice()
    }

    /// Returns the number of elements the vector can hold without reallocating.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary: Array<i32> = Array::with_capacity(10);
    /// assert_eq!(ary.capacity(), 10);
    /// ```
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// in the given `Array<T>`. The collection may reserve more space to avoid
    /// frequent reallocations. After calling reserve, capacity will be greater
    /// than or equal to `self.len() + additional`. Does nothing if capacity is
    /// already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1]);
    /// ary.reserve(10);
    /// assert!(ary.capacity() >= 11);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional);
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator
    /// may still inform the vector that there is space for a few more elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::with_capacity(10);
    /// ary.extend([1, 2, 3].iter().copied());
    /// assert_eq!(ary.capacity(), 10);
    /// ary.shrink_to_fit();
    /// assert!(ary.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the
    /// vector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// let capacity = ary.capacity();
    /// ary.clear();
    /// assert!(ary.is_empty());
    /// assert_eq!(ary.capacity(), capacity);
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns the number of elements in the vector, also referred to as its
    /// "length".
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// assert_eq!(ary.len(), 3);
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::new();
    /// assert!(ary.is_empty());
    /// ary.push(1);
    /// assert!(!ary.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a reference to an element at the index.
    ///
    /// Unlike [`Vec`], this method does not support indexing with a range.  See
    /// the [`slice`](Self::slice) method for retrieving a sub-slice from the
    /// array.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4]);
    /// assert_eq!(ary.get(1), Some(&2));
    /// assert_eq!(ary.get(3), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.0.get(index)
    }

    /// Deletes the element at the specified `index`, returning that element, or
    /// [`None`] if the `index` is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// assert_eq!(ary.delete_at(1), Some(2));
    /// assert_eq!(ary.delete_at(10), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn delete_at(&mut self, index: usize) -> Option<T> {
        if index < self.0.len() {
            Some(self.0.remove(index))
        } else {
            None
        }
    }

    /// Returns the first element from the vector, or [`None`] if the vector is
    /// empty.
    ///
    /// To retrieve a slice of the first elements in the vector, use
    /// [`first_n`](Self::first_n).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::new();
    /// assert_eq!(ary.first(), None);
    /// ary.push(1);
    /// assert_eq!(ary.first(), Some(&1));
    /// ary.push(2);
    /// assert_eq!(ary.first(), Some(&1));
    /// ```
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<&T> {
        self.0.first()
    }

    /// Returns up to `n` of the first elements from the vector, or `&[]` if the
    /// vector is empty.
    ///
    /// To retrieve only the first element in the vector, use
    /// [`first`](Self::first).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::new();
    /// assert_eq!(ary.first_n(0), &[]);
    /// assert_eq!(ary.first_n(4), &[]);
    ///
    /// ary.push(1);
    /// ary.push(2);
    /// assert_eq!(ary.first_n(0), &[]);
    /// assert_eq!(ary.first_n(4), &[1, 2]);
    ///
    /// ary.concat(&[3, 4, 5, 6, 7, 8, 9, 10]);
    /// assert_eq!(ary.first_n(0), &[]);
    /// assert_eq!(ary.first_n(4), &[1, 2, 3, 4]);
    /// ```
    #[inline]
    #[must_use]
    pub fn first_n(&self, n: usize) -> &[T] {
        self.0.get(..n).unwrap_or_else(|| &self.0[..])
    }

    /// Returns the last element from the vector, or [`None`] if the vector is
    /// empty.
    ///
    /// To retrieve a slice of the last elements in the vector, use
    /// [`last_n`](Self::last_n).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::new();
    /// assert_eq!(ary.last(), None);
    /// ary.push(1);
    /// assert_eq!(ary.last(), Some(&1));
    /// ary.push(2);
    /// assert_eq!(ary.last(), Some(&2));
    /// ```
    #[inline]
    #[must_use]
    pub fn last(&self) -> Option<&T> {
        self.0.last()
    }

    /// Returns up to `n` of the last elements from the vector, or `&[]` if the
    /// vector is empty.
    ///
    /// To retrieve only the last element in the vector, use
    /// [`last`](Self::last).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::new();
    /// assert_eq!(ary.last_n(0), &[]);
    /// assert_eq!(ary.last_n(4), &[]);
    ///
    /// ary.push(1);
    /// ary.push(2);
    /// assert_eq!(ary.last_n(0), &[]);
    /// assert_eq!(ary.last_n(4), &[1, 2]);
    ///
    /// ary.concat(&[3, 4, 5, 6, 7, 8, 9, 10]);
    /// assert_eq!(ary.last_n(0), &[]);
    /// assert_eq!(ary.last_n(4), &[7, 8, 9, 10]);
    /// ```
    #[inline]
    #[must_use]
    pub fn last_n(&self, n: usize) -> &[T] {
        let begin = self.len().checked_sub(n).unwrap_or_default();
        &self.0[begin..]
    }

    /// Returns a slice of the underlying vector that includes only the first
    /// `n` elements.
    ///
    /// If `n` is greater than or equal to the length of the vector, `&self[..]`
    /// is returned.
    ///
    /// The inverse of this operation is [`drop_n`](Self::drop_n).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4, 7, 8, 9]);
    /// assert_eq!(ary.take_n(0), &[]);
    /// assert_eq!(ary.take_n(2), &[1, 2]);
    /// assert_eq!(ary.take_n(10), &[1, 2, 4, 7, 8, 9]);
    /// ```
    #[inline]
    #[must_use]
    pub fn take_n(&self, n: usize) -> &[T] {
        self.0.get(..n).unwrap_or_else(|| &self.0[..])
    }

    /// Returns a slice of the underlying vector that excludes the first `n`
    /// elements.
    ///
    /// If `n` is greater than or equal to the length of the vector, `&[]` is
    /// returned.
    ///
    /// The inverse of this operation is [`take_n`](Self::take_n).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary = Array::from(&[1, 2, 4, 7, 8, 9]);
    /// assert_eq!(ary.drop_n(0), &[1, 2, 4, 7, 8, 9]);
    /// assert_eq!(ary.drop_n(4), &[8, 9]);
    /// assert_eq!(ary.drop_n(10), &[]);
    /// ```
    #[inline]
    #[must_use]
    pub fn drop_n(&self, n: usize) -> &[T] {
        self.0.get(n..).unwrap_or_default()
    }

    /// Removes the last element from the vector and returns it, or [`None`] if
    /// the vector is empty.
    ///
    /// To pop more than one element from the end of the vector, use
    /// [`pop_n`](Self::pop_n).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// assert_eq!(ary.pop(), Some(4));
    /// assert_eq!(ary, &[1, 2]);
    /// ```
    #[inline]
    #[must_use]
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Removes the last `n` elements from the vector.
    ///
    /// To pop a single element from the end of the vector, use
    /// [`pop`](Self::pop).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4, 7, 8, 9]);
    /// assert_eq!(ary.pop_n(0), &[]);
    /// assert_eq!(ary, &[1, 2, 4, 7, 8, 9]);
    ///
    /// assert_eq!(ary.pop_n(3), &[7, 8, 9]);
    /// assert_eq!(ary, &[1, 2, 4]);
    ///
    /// assert_eq!(ary.pop_n(100), &[1, 2, 4]);
    /// assert!(ary.is_empty());
    ///
    /// assert_eq!(ary.pop_n(1), &[]);
    /// assert!(ary.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn pop_n(&mut self, n: usize) -> Self {
        if n == 0 {
            return Self::new();
        }
        let split_at = self.len().checked_sub(n).unwrap_or_default();
        let popped = self.0.split_off(split_at);
        Self(popped)
    }

    /// Appends an element to the back of the vector.
    ///
    /// To push more than one element to the end of the vector, use
    /// [`concat`](Self::concat) or `extend`.
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the vector overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2]);
    /// ary.push(3);
    /// assert_eq!(ary, &[1, 2, 3]);
    /// ```
    #[inline]
    pub fn push(&mut self, elem: T) {
        self.0.push(elem);
    }

    /// Reverses the order of elements of the vector, in place.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// ary.reverse();
    /// assert_eq!(ary, &[4, 2, 1]);
    /// ```
    #[inline]
    pub fn reverse(&mut self) {
        self.0.reverse();
    }

    /// Removes the first element of the vector and returns it (shifting all
    /// other elements down by one). Returns [`None`] if the vector is empty.
    ///
    /// This operation is also known as "pop front".
    ///
    /// To remove more than one element from the front of the vector, use
    /// [`shift_n`](Self::shift_n).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2]);
    /// assert_eq!(ary.shift(), Some(1));
    /// assert_eq!(ary.shift(), Some(2));
    /// assert_eq!(ary.shift(), None);
    /// ```
    #[inline]
    #[must_use]
    pub fn shift(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    /// Removes the first `n` elements from the vector.
    ///
    /// To shift a single element from the front of the vector, use
    /// [`shift`](Self::shift).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4, 7, 8, 9]);
    /// assert_eq!(ary.shift_n(0), &[]);
    /// assert_eq!(ary, &[1, 2, 4, 7, 8, 9]);
    ///
    /// assert_eq!(ary.shift_n(3), &[1, 2, 4]);
    /// assert_eq!(ary, &[7, 8, 9]);
    ///
    /// assert_eq!(ary.shift_n(100), &[7, 8, 9]);
    /// assert!(ary.is_empty());
    ///
    /// assert_eq!(ary.shift_n(1), &[]);
    /// assert!(ary.is_empty());
    /// ```
    #[inline]
    #[must_use]
    pub fn shift_n(&mut self, n: usize) -> Self {
        match n {
            0 => Self::new(),
            n if n < self.0.len() => self.0.drain(..n).collect(),
            _ => {
                let shifted = self.0.split_off(0);
                Self(shifted)
            }
        }
    }

    /// Inserts an element to the front of the vector.
    ///
    /// To insert more than one element to the front of the vector, use
    /// [`unshift_n`](Self::unshift_n).
    ///
    /// This operation is also known as "prepend".
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the vector overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2]);
    /// ary.unshift(3);
    /// assert_eq!(ary, &[3, 1, 2]);
    /// ```
    #[inline]
    pub fn unshift(&mut self, elem: T) {
        self.0.insert(0, elem);
    }

    /// Return a reference to a subslice of the vector.
    ///
    /// This function always returns a slice. If the range specified by `start`
    /// and `end` overlaps the vector (even if only partially), the overlapping
    /// slice is returned. If the range does not overlap the vector, an empty
    /// slice is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let empty: Array<i32> = Array::new();
    /// assert_eq!(empty.slice(0, 0), &[]);
    /// assert_eq!(empty.slice(0, 4), &[]);
    /// assert_eq!(empty.slice(2, 4), &[]);
    ///
    /// let ary = Array::from(&[1, 2, 3]);
    /// assert_eq!(ary.slice(0, 0), &[]);
    /// assert_eq!(ary.slice(0, 4), &[1, 2, 3]);
    /// assert_eq!(ary.slice(2, 0), &[]);
    /// assert_eq!(ary.slice(2, 4), &[3]);
    /// assert_eq!(ary.slice(10, 100), &[]);
    /// ```
    #[inline]
    #[must_use]
    pub fn slice(&self, start: usize, len: usize) -> &[T] {
        if self.0.is_empty() || len == 0 {
            return &[];
        }
        if let Some(end) = start.checked_add(len) {
            self.0
                .get(start..end)
                .or_else(|| self.0.get(start..))
                .unwrap_or_default()
        } else {
            self.0.get(start..).unwrap_or_default()
        }
    }
}

impl<T> Array<T>
where
    T: Clone,
{
    /// Construct a new `Array<T>` with length `len` and all elements set to
    /// `default`. The `Array` will have capacity `len`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let ary: Array<&str> = Array::with_len_and_default(3, "spinoso");
    /// assert_eq!(ary.len(), 3);
    /// assert_eq!(ary.capacity(), 3);
    /// assert_eq!(ary, &["spinoso", "spinoso", "spinoso"]);
    /// ```
    #[inline]
    #[must_use]
    pub fn with_len_and_default(len: usize, default: T) -> Self {
        Self(vec![default; len])
    }

    /// Appends the elements of `other` to self.
    ///
    /// Slice version of `extend`. This operation is analogous to "push n".
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// ary.concat(&[7, 8, 9]);
    /// assert_eq!(ary.len(), 6);
    /// ```
    #[inline]
    pub fn concat(&mut self, other: &[T]) {
        self.0.extend_from_slice(other);
    }

    /// Prepends the elements of `other` to self.
    ///
    /// To insert one element to the front of the vector, use
    /// [`unshift`](Self::unshift).
    ///
    /// This operation is also known as "prepend".
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the vector overflows a `usize`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2]);
    /// ary.unshift_n(&[0, 5, 9]);
    /// assert_eq!(ary, &[0, 5, 9, 1, 2]);
    /// ```
    #[inline]
    pub fn unshift_n(&mut self, other: &[T]) {
        self.0.reserve(other.len());
        let mut tail = self.0.split_off(0);
        self.0.extend_from_slice(other);
        self.0.append(&mut tail);
    }
}

impl<T> Array<T>
where
    T: Copy,
{
    /// Creates a new array by repeating this array `n` times.
    ///
    /// This function will not panic. If the resulting `Array`'s capacity would
    /// overflow, [`None`] is returned.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// # fn example() -> Option<()> {
    /// let mut ary = Array::from(&[1, 2]);
    /// let repeated_ary = ary.repeat(3)?;
    /// assert_eq!(repeated_ary, &[1, 2, 1, 2, 1, 2]);
    /// # Some(())
    /// # }
    /// # example().unwrap();
    /// ```
    ///
    /// [`None`] should be returned on overflow:
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2]);
    /// let repeated_ary = ary.repeat(usize::MAX);
    /// assert_eq!(repeated_ary, None);
    /// ```
    #[must_use]
    pub fn repeat(&self, n: usize) -> Option<Self> {
        let slice = self.0.as_slice();
        if slice.len().checked_mul(n).is_some() {
            Some(Self::from(slice.repeat(n)))
        } else {
            None
        }
    }
}

impl<T> Array<T>
where
    T: Default,
{
    /// Set element at position `index` within the vector, extending the vector
    /// with `T::default()` if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2 ,4]);
    /// ary.set(1, 11);
    /// assert_eq!(ary, &[1, 11, 4]);
    /// ary.set(5, 263);
    /// assert_eq!(ary, &[1, 11, 4, 0, 0, 263]);
    ///
    /// let mut ary: Array<i32> = Array::from(&[]);
    /// ary.set(5, 11);
    /// assert_eq!(ary, &[0, 0, 0, 0, 0, 11]);
    /// ```
    #[inline]
    pub fn set(&mut self, index: usize, elem: T) {
        if let Some(cell) = self.0.get_mut(index) {
            *cell = elem;
        } else {
            let buflen = self.len();
            // index is *at least* `buflen`, so this calculation never underflows
            // and ensures we allocate an additional slot.
            let additional = (index - buflen).checked_add(1).expect("capacity overflow");
            self.0.reserve(additional);
            self.0.resize_with(index, T::default);
            self.0.push(elem);
        }
    }

    /// Insert element at position `start` within the vector and remove the
    /// following `drain` elements. If `start` is out of bounds, the vector will
    /// be extended with `T::default()`.
    ///
    /// This method sets a slice of the `Array` to a single element, including
    /// the zero-length slice. It is similar in intent to calling
    /// [`Vec::splice`] with a one-element iterator.
    ///
    /// `set_with_drain` will only drain up to the end of the vector.
    ///
    /// To set a single element without draining, use [`set`](Self::set).
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// ary.set_with_drain(1, 0, 10);
    /// assert_eq!(ary, &[1, 10, 2, 4]);
    /// ary.set_with_drain(2, 5, 20);
    /// assert_eq!(ary, &[1, 10, 20]);
    /// ary.set_with_drain(5, 5, 30);
    /// assert_eq!(ary, &[1, 10, 20, 0, 0, 30]);
    /// ```
    #[inline]
    pub fn set_with_drain(&mut self, start: usize, drain: usize, elem: T) -> usize {
        let buflen = self.0.len();
        let drained = cmp::min(buflen.checked_sub(start).unwrap_or_default(), drain);

        if let Some(cell) = self.0.get_mut(start) {
            match drain {
                0 => self.0.insert(start, elem),
                1 => *cell = elem,
                _ => {
                    *cell = elem;
                    let drain_end_idx = cmp::min(start.saturating_add(drain), buflen);
                    self.0.drain(start.saturating_add(1)..drain_end_idx);
                }
            }
        } else {
            // start is *at least* `buflen`, so this calculation never underflows
            // and ensures we allocate an additional slot.
            let additional = (start - buflen).checked_add(1).expect("capacity overflow");
            self.0.reserve(additional);
            self.0.resize_with(start, T::default);
            self.0.push(elem);
        }

        drained
    }
}

impl<T> Array<T>
where
    T: Default + Clone,
{
    /// Insert the elements from a slice at a position `index` in the vector,
    /// extending the vector with `T::default()` if `index` is out of bounds.
    ///
    /// This method is similar to [`Vec::splice`] when called with a zero-length
    /// range.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// ary.insert_slice(1, &[7, 8, 9]);
    /// assert_eq!(ary, &[1, 7, 8, 9, 2, 4]);
    /// ary.insert_slice(8, &[100, 200]);
    /// assert_eq!(ary, &[1, 7, 8, 9, 2, 4, 0, 0, 100, 200]);
    /// ```
    #[inline]
    pub fn insert_slice(&mut self, index: usize, values: &[T]) {
        if let Some(overflow) = index.checked_sub(self.0.len()) {
            let additional = overflow.checked_add(values.len()).expect("capacity overflow");
            self.0.reserve(additional);
            self.0.resize_with(index, T::default);
        } else {
            self.0.reserve(values.len());
        }
        if index == self.0.len() {
            self.0.extend_from_slice(values);
        } else {
            let mut tail = self.0.split_off(index);
            self.0.extend_from_slice(values);
            self.0.append(&mut tail);
        }
    }

    /// Insert the elements from a slice at a position `index` in the vector and
    /// remove the following `drain` elements. The vector is extended with
    /// `T::default()` if `index` is out of bounds.
    ///
    /// This method is similar to [`Vec::splice`] when called with a
    /// nonzero-length range.
    ///
    /// When called with `drain == 0`, this method is equivalent to
    /// [`insert_slice`](Self::insert_slice).
    ///
    /// If `drain >= src.len()` or the tail of the vector is replaced, this
    /// method is efficient. Otherwise, a temporary buffer is used to move the
    /// elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use spinoso_array::Array;
    /// let mut ary = Array::from(&[1, 2, 4]);
    /// ary.set_slice(1, 5, &[7, 8, 9]);
    /// assert_eq!(ary, &[1, 7, 8, 9]);
    /// ary.set_slice(6, 1, &[100, 200]);
    /// assert_eq!(ary, &[1, 7, 8, 9, 0, 0, 100, 200]);
    /// ```
    #[inline]
    pub fn set_slice(&mut self, index: usize, drain: usize, values: &[T]) -> usize {
        let buflen = self.0.len();
        let drained = cmp::min(buflen.checked_sub(index).unwrap_or_default(), drain);

        if let Some(overflow) = index.checked_sub(self.0.len()) {
            let additional = overflow.saturating_add(values.len());
            self.0.reserve(additional);
            self.0.resize_with(index, T::default);
        }
        if index == self.0.len() {
            self.0.extend_from_slice(values);
        } else {
            self.0
                .splice(index..index.saturating_add(drained), values.iter().cloned());
        }

        drained
    }
}

#[cfg(test)]
mod test {
    use crate::array::vec::{Array, RawParts};

    // `insert_slice`

    #[test]
    fn non_empty_array_insert_slice_end_empty() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(5, &[]);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_insert_slice_out_of_bounds_empty() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(10, &[]);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn non_empty_array_insert_slice_interior_empty() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(2, &[]);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_insert_slice_begin_empty() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(0, &[]);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn empty_array_insert_slice_end_empty() {
        let mut ary = Array::<i32>::new();
        ary.insert_slice(0, &[]);
        assert_eq!(ary, []);
    }

    #[test]
    fn empty_array_insert_slice_out_of_bounds_empty() {
        let mut ary = Array::<i32>::new();
        ary.insert_slice(10, &[]);
        assert_eq!(ary, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn empty_array_insert_slice_begin_empty() {
        let mut ary = Array::<i32>::new();
        ary.insert_slice(0, &[]);
        assert_eq!(ary, []);
    }

    #[test]
    fn non_empty_array_insert_slice_end() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(5, &[8, 9, 10]);
        assert_eq!(ary, [1, 2, 3, 4, 5, 8, 9, 10]);
    }

    #[test]
    fn non_empty_array_insert_slice_out_of_bounds() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(10, &[8, 9, 10]);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 8, 9, 10]);
    }

    #[test]
    fn non_empty_array_insert_slice_interior() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(2, &[8, 9, 10]);
        assert_eq!(ary, [1, 2, 8, 9, 10, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_insert_slice_begin() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        ary.insert_slice(0, &[8, 9, 10]);
        assert_eq!(ary, [8, 9, 10, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn empty_array_insert_slice_end() {
        let mut ary = Array::<i32>::new();
        ary.insert_slice(0, &[8, 9, 10]);
        assert_eq!(ary, [8, 9, 10]);
    }

    #[test]
    fn empty_array_insert_slice_out_of_bounds() {
        let mut ary = Array::<i32>::new();
        ary.insert_slice(10, &[8, 9, 10]);
        assert_eq!(ary, [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 9, 10]);
    }

    #[test]
    fn empty_array_insert_slice_begin() {
        let mut ary = Array::<i32>::new();
        ary.insert_slice(0, &[8, 9, 10]);
        assert_eq!(ary, [8, 9, 10]);
    }

    // `set_slice`

    #[test]
    fn non_empty_array_set_slice_end_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_end_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_end_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_end_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 5, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 2, &[]);
        assert_eq!(drained, 2);
        assert_eq!(ary, [1, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_empty_drain_greater_than_insert_length_to_end() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 4, &[]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [1]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_empty_drain_greater_than_insert_length_overrun_end() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 10, &[]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [1]);
    }

    #[test]
    fn non_empty_array_set_slice_end_non_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 0, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_end_non_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 2, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_end_non_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 3, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_end_non_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(5, 5, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_non_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 0, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_non_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 2, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_non_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 3, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_out_of_bounds_non_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(10, 5, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 2, 3, 4, 5, 0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 0, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [1, 7, 8, 9, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 2, &[7, 8, 9]);
        assert_eq!(drained, 2);
        assert_eq!(ary, [1, 7, 8, 9, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(1, 3, &[7, 8, 9]);
        assert_eq!(drained, 3);
        assert_eq!(ary, [1, 7, 8, 9, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5, 6]);
        let drained = ary.set_slice(1, 4, &[7, 8, 9]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [1, 7, 8, 9, 6]);
        assert_eq!(ary.len(), 5);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_equal_to_insert_length_to_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(1, 3, &[7, 8, 9]);
        assert_eq!(drained, 3);
        assert_eq!(ary, [1, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_greater_than_insert_length_to_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(1, 10, &[7, 8, 9]);
        assert_eq!(drained, 3);
        assert_eq!(ary, [1, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_less_than_insert_length_overrun_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(3, 2, &[7, 8, 9]);
        assert_eq!(drained, 1);
        assert_eq!(ary, [1, 2, 3, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_equal_to_insert_length_overrun_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(3, 3, &[7, 8, 9]);
        assert_eq!(drained, 1);
        assert_eq!(ary, [1, 2, 3, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_interior_non_empty_drain_greater_than_insert_length_overrun_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(3, 10, &[7, 8, 9]);
        assert_eq!(drained, 1);
        assert_eq!(ary, [1, 2, 3, 7, 8, 9]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_0() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(0, 0, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [7, 8, 9, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_less_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(0, 2, &[7, 8, 9]);
        assert_eq!(drained, 2);
        assert_eq!(ary, [7, 8, 9, 3, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_equal_to_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5]);
        let drained = ary.set_slice(0, 3, &[7, 8, 9]);
        assert_eq!(drained, 3);
        assert_eq!(ary, [7, 8, 9, 4, 5]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_greater_than_insert_length() {
        let mut ary = Array::from([1, 2, 3, 4, 5, 6]);
        let drained = ary.set_slice(0, 4, &[7, 8, 9]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [7, 8, 9, 5, 6]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_equal_to_insert_length_to_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(0, 4, &[7, 8, 9, 10]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [7, 8, 9, 10]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_greater_than_insert_length_to_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(0, 10, &[7, 8, 9, 10]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [7, 8, 9, 10]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_less_than_insert_length_overrun_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(0, 4, &[7, 8, 9, 10, 11]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [7, 8, 9, 10, 11]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_equal_to_insert_length_overrun_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(0, 5, &[7, 8, 9, 10, 11]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [7, 8, 9, 10, 11]);
    }

    #[test]
    fn non_empty_array_set_slice_begin_non_empty_drain_greater_than_insert_length_overrun_tail() {
        let mut ary = Array::from([1, 2, 3, 4]);
        let drained = ary.set_slice(0, 10, &[7, 8, 9, 10, 11]);
        assert_eq!(drained, 4);
        assert_eq!(ary, [7, 8, 9, 10, 11]);
    }

    #[test]
    fn empty_array_set_slice_non_empty_drain_0() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 0, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_non_empty_drain_less_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 1, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_non_empty_drain_equal_to_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 3, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_begin_non_empty_drain_greater_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 10, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_non_empty_drain_0() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 0, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_non_empty_drain_less_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 1, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_non_empty_drain_equal_to_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 3, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_non_empty_drain_greater_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 10, &[7, 8, 9]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0, 7, 8, 9]);
    }

    #[test]
    fn empty_array_set_slice_empty_drain_0() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, []);
    }

    #[test]
    fn empty_array_set_slice_empty_drain_less_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, []);
    }

    #[test]
    fn empty_array_set_slice_empty_drain_equal_to_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, []);
    }

    #[test]
    fn empty_array_set_slice_begin_empty_drain_greater_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(0, 10, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, []);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_empty_drain_0() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 0, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_empty_drain_less_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 1, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_empty_drain_equal_to_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 3, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn empty_array_set_slice_out_of_bounds_empty_drain_greater_than_insert_length() {
        let mut ary = Array::<i32>::new();
        let drained = ary.set_slice(5, 10, &[]);
        assert_eq!(drained, 0);
        assert_eq!(ary, [0, 0, 0, 0, 0]);
    }

    #[test]
    fn into_raw_parts_from_raw_parts_round_trip_empty_no_alloc() {
        let ary = Array::<i32>::new();
        let RawParts { ptr, length, capacity } = ary.into_raw_parts();
        let ary = unsafe { Array::from_raw_parts(RawParts { ptr, length, capacity }) };
        assert_eq!(ary.len(), 0);
        assert_eq!(ary.capacity(), 0);
    }

    #[test]
    fn into_raw_parts_from_raw_parts_round_trip_empty_with_capacity() {
        let ary = Array::<i32>::with_capacity(100);
        let RawParts { ptr, length, capacity } = ary.into_raw_parts();
        let ary = unsafe { Array::from_raw_parts(RawParts { ptr, length, capacity }) };
        assert_eq!(ary.len(), 0);
        assert_eq!(ary.capacity(), 100);
    }

    #[test]
    fn into_raw_parts_from_raw_parts_round_trip_assoc() {
        let ary = Array::<i32>::assoc(1, 2);
        let RawParts { ptr, length, capacity } = ary.into_raw_parts();
        let ary = unsafe { Array::from_raw_parts(RawParts { ptr, length, capacity }) };
        assert_eq!(ary.len(), 2);
        assert_eq!(ary.capacity(), 2);
        assert_eq!(ary, [1, 2]);
    }

    #[test]
    fn into_raw_parts_from_raw_parts_round_trip_from_slice() {
        let ary = Array::<i32>::from(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
        let RawParts { ptr, length, capacity } = ary.into_raw_parts();
        let ary = unsafe { Array::from_raw_parts(RawParts { ptr, length, capacity }) };
        assert_eq!(ary.len(), 10);
        assert!(ary.capacity() >= 10);
        assert_eq!(ary, [1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    }
}
