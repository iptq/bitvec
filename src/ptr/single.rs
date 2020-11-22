use crate::{
	access::BitAccess,
	index::BitIdx,
	mutability::{
		Const,
		Mut,
		Mutability,
	},
	order::BitOrder,
	ptr::{
		Address,
		AddressError,
	},
	store::BitStore,
};

use core::{
	any::TypeId,
	cmp,
	convert::{
		TryFrom,
		TryInto,
	},
	marker::PhantomData,
	ptr::NonNull,
};

/** An opaque non-null pointer to a single bit in a memory element.

# Original

[`*const bool` and `*mut bool`](https://doc.rust-lang.org/std/primitive.pointer.html)

# API Differences

All pointer types in `bitvec` take type parameters to determine the type of the
underlying memory element and the ordering of bits within it.

Additionally, the types corresponding to raw pointers take a third type
parameter to encode mutability, rather than follow the standard library
convention of having two near-equivalent sibling types.
**/
pub struct BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	M: Mutability,
{
	/// Address of the referent element.
	addr: Address<T, M>,
	/// Index of the bit within the referent element.
	head: BitIdx<T::Mem>,
	/// The ordering used to map `self.head` to an electrical position.
	_ord: PhantomData<O>,
}

impl<O, T, M> BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	M: Mutability,
{
	pub(crate) const DANGLING: Self = Self {
		addr: Address::DANGLING,
		head: BitIdx::ZERO,
		_ord: PhantomData,
	};

	/// Constructs a new single-bit pointer from an element address and a bit
	/// index.
	///
	/// # Parameters
	///
	/// - `addr`: Something that can be used as a memory address. This can be a
	///   reference, a raw pointer, or a [`NonNull`] pointer.
	/// - `head`: An index of a bit within the memory element at `addr`.
	///
	/// # Returns
	///
	/// An opaque pointer to a single bit within a memory element. This cannot
	/// be cast to any raw pointer type. If `addr` is null, or incorrectly
	/// aligned for `T`, this returns an error rather than a pointer.
	///
	/// [`NonNull`]: core::ptr::NonNull
	pub fn new<A>(
		addr: A,
		head: BitIdx<T::Mem>,
	) -> Result<Self, AddressError<T>>
	where
		A: TryInto<Address<T, M>, Error = AddressError<T>>,
	{
		let addr = addr.try_into()?;
		Ok(unsafe { Self::new_unchecked(addr, head) })
	}

	/// Constructs a new single-bit pointer from an element address and bit
	/// index, without checking that the address is correctly usable.
	///
	/// # Parameters
	///
	/// - `addr`: Something that can be used as a memory address. This can be a
	///   reference, a raw pointer, or a [`NonNull`] pointer.
	/// - `head`: An index of a bit within the memory element at `addr`.
	///
	/// # Returns
	///
	/// An opaque pointer to a single bit within a memory element. This cannot
	/// be cast to any raw pointer type.
	///
	/// # Safety
	///
	/// `addr` is not inspected for correctness, and
	pub unsafe fn new_unchecked<A>(addr: A, head: BitIdx<T::Mem>) -> Self
	where A: Into<Address<T, M>> {
		let addr = addr.into();
		Self {
			addr,
			head,
			_ord: PhantomData,
		}
	}

	/// Decomposes the pointer into its element address and bit index.
	pub fn raw_parts(self) -> (Address<T, M>, BitIdx<T::Mem>) {
		(self.addr, self.head)
	}

	/// Reads the referent bit out of memory.
	///
	/// # Safety
	///
	/// This is `unsafe`, because there is no requirement that the pointer
	/// target validly initialized, or even allocated, memory. You must
	/// guarantee that the referent element is allocated, initialized, and not
	/// in aliasing violations, before calling this.
	pub unsafe fn read(self) -> bool {
		(&*self.addr.to_const()).get_bit::<O>(self.head)
	}
}

impl<O, T> BitPtr<O, T, Mut>
where
	O: BitOrder,
	T: BitStore,
{
	/// Writes a bit into the referent slot.
	///
	/// # Safety
	///
	/// This is `unsafe`, because there is no requirement that the pointer
	/// target validily initialized, or even allocated, memory. You must
	/// guarantee that the referent element is allocated, initialized, and not
	/// in aliasing violations, before calling this.
	pub unsafe fn write(self, value: bool) {
		(&*self.addr.to_access()).write_bit::<O>(self.head, value)
	}
}

impl<O, T, M> Clone for BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	M: Mutability,
{
	#[inline(always)]
	fn clone(&self) -> Self {
		*self
	}
}

impl<O, T, M> Eq for BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	M: Mutability,
{
}

impl<O, T, M> Ord for BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	M: Mutability,
{
	fn cmp(&self, other: &Self) -> cmp::Ordering {
		self.partial_cmp(&other)
			.expect("BitPtr should have a total ordering")
	}
}

impl<O, T, U, M, N> PartialEq<BitPtr<O, U, N>> for BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	U: BitStore,
	M: Mutability,
	N: Mutability,
{
	fn eq(&self, other: &BitPtr<O, U, N>) -> bool {
		if TypeId::of::<T::Mem>() != TypeId::of::<U::Mem>() {
			return false;
		}
		self.addr.value() == other.addr.value()
			&& self.head.value() == other.head.value()
	}
}

impl<O, T, U, M, N> PartialOrd<BitPtr<O, U, N>> for BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	U: BitStore,
	M: Mutability,
	N: Mutability,
{
	fn partial_cmp(&self, other: &BitPtr<O, U, N>) -> Option<cmp::Ordering> {
		if TypeId::of::<T::Mem>() != TypeId::of::<U::Mem>() {
			return None;
		}
		match (self.addr.value()).cmp(&(other.addr.value())) {
			cmp::Ordering::Equal => {
				self.head.value().partial_cmp(&other.head.value())
			},
			ord => return Some(ord),
		}
	}
}

impl<O, T> From<&T> for BitPtr<O, T, Const>
where
	O: BitOrder,
	T: BitStore,
{
	fn from(src: &T) -> Self {
		Self {
			addr: src.into(),
			..Self::DANGLING
		}
	}
}

impl<O, T> From<&mut T> for BitPtr<O, T, Mut>
where
	O: BitOrder,
	T: BitStore,
{
	fn from(src: &mut T) -> Self {
		Self {
			addr: src.into(),
			..Self::DANGLING
		}
	}
}

impl<O, T> From<NonNull<T>> for BitPtr<O, T, Mut>
where
	O: BitOrder,
	T: BitStore,
{
	fn from(src: NonNull<T>) -> Self {
		Self {
			addr: src.into(),
			..Self::DANGLING
		}
	}
}

impl<O, T> TryFrom<*const T> for BitPtr<O, T, Const>
where
	O: BitOrder,
	T: BitStore,
{
	type Error = AddressError<T>;

	fn try_from(src: *const T) -> Result<Self, Self::Error> {
		Ok(Self {
			addr: src.try_into()?,
			..Self::DANGLING
		})
	}
}

impl<O, T> TryFrom<*mut T> for BitPtr<O, T, Mut>
where
	O: BitOrder,
	T: BitStore,
{
	type Error = AddressError<T>;

	fn try_from(src: *mut T) -> Result<Self, Self::Error> {
		Ok(Self {
			addr: src.try_into()?,
			..Self::DANGLING
		})
	}
}

impl<O, T, M> Copy for BitPtr<O, T, M>
where
	O: BitOrder,
	T: BitStore,
	M: Mutability,
{
}