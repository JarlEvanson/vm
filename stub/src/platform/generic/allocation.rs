//! Implementation of the memory allocation API provided to the remainder of `revm-stub`.

use core::{mem, ptr::NonNull};

use sync::Spinlock;

use crate::platform::generic::platform;

/// The head of the linked list of allocated regions.
static HEAD: Spinlock<Option<SendPtr<AllocationRecord>>> = Spinlock::new(None);

/// Allocates a region of memory of `size` bytes aligned to `alignment`.
pub fn allocate(size: usize, alignment: usize) -> Option<Allocation> {
    let mut head = HEAD.lock();

    let allocation_record_ptr = platform().allocate(
        mem::size_of::<AllocationRecord>(),
        mem::align_of::<AllocationRecord>(),
    )?;

    let Some(allocation) = platform().allocate(size, alignment) else {
        // SAFETY:
        //
        // We are not using the memory region indicated by `allocation_record_ptr`.
        unsafe {
            platform().deallocate(
                allocation_record_ptr,
                mem::size_of::<AllocationRecord>(),
                mem::align_of::<AllocationRecord>(),
            )
        }

        // Drop `head` early to enable printing mechanisms that may require allocation.
        drop(head);

        crate::trace!("allocate({size}, {alignment}) -> None");
        return None;
    };

    // SAFETY:
    //
    // The region of memory pointed to by `allocation_record_ptr` was just allocated and is under
    // the exclusive control of this module.
    unsafe {
        allocation_record_ptr
            .cast::<AllocationRecord>()
            .write(AllocationRecord {
                ptr: allocation,
                size,
                alignment,
                next: head.as_ref().map(|wrapper| wrapper.0),
            })
    }

    // Drop `head` early to enable printing mechanisms that may require allocation.
    *head = Some(SendPtr(allocation_record_ptr.cast::<AllocationRecord>()));
    drop(head);

    crate::trace!("allocate({size}, {alignment}) -> {:p}", allocation.as_ptr());
    Some(Allocation {
        ptr: allocation,
        size,
        alignment,
    })
}

/// Deallocates the memory referenced by `ptr`.
///
/// # Safety
///
/// The `size` and `alignment` parameters must match the parameters utilized when the memory
/// referenced by `ptr` was allocated by a call to [`allocate()`]. `ptr` must describe a currently
/// allocated block of memory.
#[expect(
    clippy::missing_panics_doc,
    reason = "panicking only occurs if safety invariants are violated"
)]
pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize, alignment: usize) {
    crate::trace!("deallocate({ptr:p}, {size}, {alignment})");

    let mut head = HEAD.lock();
    let mut current = head.as_ref().map(|wrapper| wrapper.0);
    let mut prev: Option<NonNull<AllocationRecord>> = None;

    while let Some(node) = current {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
        // protected by the [`HEAD`] lock.
        let record = unsafe { node.as_ref() };
        if record.ptr == ptr {
            if record.size != size || record.alignment != alignment {
                // Drop `head` to enable its use in the panic handler.
                drop(head);
                panic!("allocation metadata mismatch on deallocate()");
            }

            let next = record.next;
            match prev {
                // SAFETY:
                //
                // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
                // protected by the [`HEAD`] lock.
                Some(mut p) => unsafe { p.as_mut().next = next },
                None => *head = next.map(SendPtr),
            }

            // SAFETY:
            //
            // We have removed all references to this memory block and it will not be used after
            // this.
            unsafe {
                platform().deallocate(
                    node.cast::<u8>(),
                    mem::size_of::<AllocationRecord>(),
                    mem::align_of::<AllocationRecord>(),
                )
            }

            // SAFETY:
            //
            // The invariants of this function and the validation confirm that this region of
            // memory is safe to deallocate.
            unsafe { platform().deallocate(ptr, size, alignment) }

            // Force the [`Spinlock`] to be held until the end of this function.
            drop(head);
            return;
        }

        prev = current;
        current = record.next;
    }

    // Drop `head` to enable its use in the panic handler.
    drop(head);
    panic!("attempted to deallocate unknown pointer");
}

/// Deallocates all outstanding memory allocations.
///
/// # Safety
///
/// All memory allocated by calls to [`allocate()`] must not be accessed after this call begins.
pub unsafe fn deallocate_all() {
    let mut head = HEAD.lock();

    let mut current = head.as_ref().map(|wrapper| wrapper.0);
    while let Some(node) = current {
        // SAFETY:
        //
        // All `Some` values point to a valid [`AllocationRecord`] and this linked list is
        // protected by the [`HEAD`] lock.
        let record = unsafe { node.as_ref() };

        // SAFETY:
        //
        // The invariants of this function and the validation confirm that this region of
        // memory is safe to deallocate.
        unsafe { platform().deallocate(record.ptr, record.size, record.alignment) }

        let next = record.next;

        // SAFETY:
        //
        // We have removed all references to this memory block and it will not be used after
        // this.
        unsafe {
            platform().deallocate(
                node.cast::<u8>(),
                mem::size_of::<AllocationRecord>(),
                mem::align_of::<AllocationRecord>(),
            )
        }

        current = next;
    }

    // Clear validation list and release spinlock.
    *head = None;
    drop(head)
}

/// Wrapper around a region of memory allocated with [`allocate()`].
///
/// This automatically frees the memory.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Allocation {
    /// The address of the start of this region of memory.
    ptr: NonNull<u8>,
    /// The size value utilized to allocate this region of memory.
    size: usize,
    /// The alignment utilized to allocate this region of memory.
    alignment: usize,
}

impl Allocation {
    /// Returns the address of the start of the region of memory controlled by this [`Allocation`].
    pub const fn ptr(&self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Returns the address of the start of the region of memory controlled by this [`Allocation`]
    /// as a [`NonNull`] pointer.
    pub const fn ptr_nonnull(&self) -> NonNull<u8> {
        self.ptr
    }

    /// Returns the number of bytes requested when allocating the region of memory controlled by
    /// this [`Allocation`].
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Returns the alignment utilized when allocating the region of memory controlled by this
    /// [`Allocation`].
    pub const fn alignment(&self) -> usize {
        self.alignment
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // SAFETY:
        //
        // The region of memory indicated by `ptr` and allocated with `size` and `alignment` is
        // under the exclusive control of [`deallocate()`].
        unsafe { deallocate(self.ptr, self.size, self.alignment) }
    }
}

/// The record of an allocation.
struct AllocationRecord {
    /// The allocated pointer.
    ptr: NonNull<u8>,
    /// The size, in bytes, of that allocation.
    size: usize,
    /// The alignment, in bytes, of that allocation.
    alignment: usize,
    /// The next [`AllocationRecord`] in the linked list.
    next: Option<NonNull<AllocationRecord>>,
}

// SAFETY:
//
// [`AllocationRecord`]s are safe to send to other threads, since they don't expose anything
// that requires a single thread.
unsafe impl Send for AllocationRecord {}

// SAFETY:
//
// [`AllocationRecord`]s are safe to send to other threads, since they don't expose anything
// that requires a single thread.
unsafe impl Sync for AllocationRecord {}

/// Sync wrapper.
#[repr(transparent)]
struct SendPtr<T>(NonNull<T>);

// SAFETY:
//
// [`SendPtr<T>`]s are safe to send to other threads, since they don't expose anything
// that requires a single thread.
unsafe impl<T> Send for SendPtr<T> {}
// SAFETY:
//
// [`SendPtr<T>`]s are safe to read from other threads, since they don't expose anything that
// requires a single thread.
unsafe impl<T> Sync for SendPtr<T> {}
