//! An fixed-size allocator based on Linux's SLAB allocator.
//!
//! # Locking Order
//!
//! All operations must lock the [`SlabCache`] first, then hold only a single lock.

use core::{
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
    slice,
};

use conversion::{u32_to_usize, usize_to_u32_strict, usize_to_u64};
use memory::address::VirtualAddress;
use sync::{ControlledModificationCell, Spinlock};

use crate::memory::{
    page_frame_size,
    phys::{allocate_frames, structs::Frame},
    virt::{Permissions, map},
};

/// The minimum size class that the SLAB allocator supports.
const MIN_SIZE_CLASS: usize = mem::size_of::<usize>() * 2;
/// The number of [`Page`][p] [`Frame`]s that each [`Slab`] resides within.
///
/// [p]: crate::memory::virt::structs::Page
const SLAB_PAGE_FRAME_COUNT: usize = 1;

/// The maximum size class the SLAB allocator subsystem supports.
pub static MAX_SIZE_CLASS: ControlledModificationCell<usize> = ControlledModificationCell::new(0);

/// An ordered slice of [`SlabCache`]s.
static SLAB_CACHES: ControlledModificationCell<&'static [Spinlock<SlabCache>]> =
    ControlledModificationCell::new(&[]);

/// Initializes the SLAB allocator subsystem.
///
/// # Panics
///
/// Panics if the `page_frame_size()` is too small or if frame allocations and the mapping thereof fail.
///
/// # Safety
///
/// This function must be called before any slab allocator functions are called but after the
/// physical memory and virtual memory subsystems are initialized.
pub unsafe fn initialize() {
    let page_frame_size = page_frame_size();
    assert!(
        page_frame_size >= 512,
        "minimum page frame size is 512 bytes"
    );

    let slab_allocation_size = page_frame_size.strict_mul(SLAB_PAGE_FRAME_COUNT);
    let max_size_class = slab_allocation_size / 8;
    let min_size_class = MIN_SIZE_CLASS;

    let mut size_class_count = 0;
    let mut size_class_tracker = min_size_class;
    while size_class_tracker <= max_size_class {
        size_class_tracker = size_class_tracker.strict_mul(2);
        size_class_count += 1;
    }

    let slab_cache_size = size_class_count * mem::size_of::<Spinlock<SlabCache>>();
    let page_frame_count = slab_cache_size.div_ceil(page_frame_size);

    let frame_range = allocate_frames(
        usize_to_u64(page_frame_count),
        usize_to_u64(mem::align_of::<Spinlock<SlabCache>>()),
    )
    .expect("failed to allocate SlabCache frames");
    let page_range =
        map(frame_range, Permissions::ReadWrite).expect("failed to map SlabCache frames");

    // SAFETY:
    //
    // The referenced pages are under the exclusive control of this function and [`MaybeUninit`]
    // does not expect initialized memory.
    let slabs = unsafe {
        slice::from_raw_parts_mut(
            ptr::with_exposed_provenance_mut::<MaybeUninit<Spinlock<SlabCache>>>(
                page_range.start().start_address().value(),
            ),
            size_class_count,
        )
    };

    for (index, slab) in slabs.iter_mut().enumerate() {
        slab.write(Spinlock::new(SlabCache {
            object_size: min_size_class * 2usize.pow(usize_to_u32_strict(index)),
            empty: None,
            partial: None,
            full: None,
        }));
    }

    // SAFETY:
    //
    // The referenced pages are under the exclusive control of this function and the slabs are
    // initialized.
    let slabs = unsafe {
        slice::from_raw_parts(
            ptr::with_exposed_provenance_mut::<Spinlock<SlabCache>>(
                page_range.start().start_address().value(),
            ),
            size_class_count,
        )
    };

    // SAFETY:
    //
    // This function happens before any SLAB subystem call has occurred.
    unsafe { *SLAB_CACHES.get_mut() = slabs }

    // SAFETY:
    //
    // This function happens before any SLAB subystem call has occurred.
    unsafe { *MAX_SIZE_CLASS.get_mut() = max_size_class };
}

/// Allocates a region of memory of `size` bytes.
pub fn allocate(size: usize) -> Option<NonNull<u8>> {
    let slab_cache_index = calculate_cache_index(size);
    let slab_cache_ref = &SLAB_CACHES.get()[slab_cache_index];
    let mut slab_cache = slab_cache_ref.lock();

    if let Some(partial) = slab_cache.partial {
        // Try to allocate from partial slabs first.

        // SAFETY:
        //
        // The [`SlabCache`] is locked so `partial` will not move during this operation and
        // initialized [`Slab`]s are always read utilizing immutable references.
        let partial_slab = unsafe { partial.as_ref() };
        let mut partial_slab_lock = partial_slab.inner.lock();

        let ptr = partial_slab_lock.allocate();
        if partial_slab_lock.full() {
            let partial_slab_lock_next = partial_slab_lock.next;

            partial_slab_lock.prev = None;
            partial_slab_lock.next = slab_cache.full;
            drop(partial_slab_lock);

            // Remove full slab from partial slab list.
            slab_cache.partial = partial_slab_lock_next;
            if let Some(next_slab) = partial_slab_lock_next {
                // SAFETY:
                //
                // The [`SlabCache`] is locked so `partial.next` will not move during this
                // operation and initialized [`Slab`]s are always read utilizing immutable
                // references.
                unsafe { next_slab.as_ref().inner.lock().prev = None }
            }

            // Add the new full slab to the start of the full slab list.
            if let Some(next_slab) = slab_cache.full {
                // SAFETY:
                //
                // The [`SlabCache`] is locked so `partial.next` will not move during this
                // operation and initialized [`Slab`]s are always read utilizing immutable
                // references.
                unsafe { next_slab.as_ref().inner.lock().prev = Some(partial) }
            }

            slab_cache.full = Some(partial);
        }

        Some(ptr)
    } else if let Some(empty) = slab_cache.empty {
        // Try to allocate from cached empty slabs.

        // SAFETY:
        //
        // The [`SlabCache`] is locked so `empty` will not move during this operation and
        // initialized [`Slab`]s are always read utilizing immutable references.
        let empty_slab = unsafe { empty.as_ref() };
        let mut empty_slab_lock = empty_slab.inner.lock();

        let ptr = empty_slab_lock.allocate();

        let empty_slab_lock_next = empty_slab_lock.next;
        empty_slab_lock.prev = None;
        empty_slab_lock.next = slab_cache.partial;
        drop(empty_slab_lock);

        // Remove the partial slab from the empty slab list.
        slab_cache.empty = empty_slab_lock_next;
        if let Some(next_slab) = empty_slab_lock_next {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `empty.next` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next_slab.as_ref().inner.lock().prev = None }
        }

        // Add the new partial slab to the start of the partial slab list.
        if let Some(next_slab) = slab_cache.partial {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `empty.next` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next_slab.as_ref().inner.lock().prev = Some(empty) }
        }

        slab_cache.partial = Some(empty);

        Some(ptr)
    } else {
        // Otherwise, we must allocate a new [`Slab`].

        let new = Slab::alloc_slab(slab_cache_ref, slab_cache.object_size)?;

        // SAFETY:
        //
        // The [`SlabCache`] is locked so `new` will not move during this operation and
        // initialized [`Slab`]s are always read utilizing immutable references.
        let new_slab = unsafe { new.as_ref() };
        let mut new_slab_lock = new_slab.inner.lock();

        let ptr = new_slab_lock.allocate();

        new_slab_lock.prev = None;
        new_slab_lock.next = slab_cache.partial;
        drop(new_slab_lock);

        // Add the new partial slab to the start of the partial slab list.
        if let Some(next_slab) = slab_cache.partial {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `empty.next` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next_slab.as_ref().inner.lock().prev = Some(new) }
        }

        slab_cache.partial = Some(new);

        Some(ptr)
    }
}

/// Deallocates the provided `ptr` that refers to `size` bytes.
///
/// # Safety
///
/// The provided `ptr` must be an active allocation and must not be used after the start of this
/// function. `size` must be accurate.
pub unsafe fn deallocate(ptr: NonNull<u8>, _size: usize) {
    let page_frame_count = SLAB_PAGE_FRAME_COUNT;
    let alignment = page_frame_count.strict_mul(page_frame_size());

    let virt_addr = VirtualAddress::new(ptr.as_ptr().addr());
    let slab_address = virt_addr.align_down(alignment);
    let slab_ptr = ptr::without_provenance::<Slab>(slab_address.value());

    // SAFETY:
    //
    // Initialized [`Slab`]s are always read utilizing immutable references and partial [`Slab`]s
    // are never moved or destroyed.
    let active_slab = unsafe { &*slab_ptr };
    let slab_cache = active_slab.cache;

    let mut slab_cache = slab_cache.lock();

    let mut active_slab_lock = active_slab.inner.lock();
    let was_full = active_slab_lock.full();

    // SAFETY:
    //
    // According to the invariants of this function, `ptr` will not be used again and is an active
    // slab object.
    unsafe { active_slab_lock.deallocate(ptr.as_ptr()) }
    if active_slab_lock.empty() {
        let active_slab_lock_prev = active_slab_lock.prev;
        let active_slab_lock_next = active_slab_lock.next;

        active_slab_lock.prev = None;
        active_slab_lock.next = slab_cache.empty;
        drop(active_slab_lock);

        // Remove the newly empty slab from the partial slab list.
        if let Some(prev) = active_slab_lock_prev {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `active_slab_lock.prev` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { prev.as_ref().inner.lock().next = active_slab_lock_next }
        } else {
            slab_cache.partial = active_slab_lock_next;
        };

        if let Some(next) = active_slab_lock_next {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `active_slab_lock.prev` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next.as_ref().inner.lock().prev = active_slab_lock_prev }
        }

        // Add the new empty slab to the start of the empty slab list.
        if let Some(next_slab) = slab_cache.empty {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `empty.next` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next_slab.as_ref().inner.lock().prev = Some(NonNull::from_ref(active_slab)) }
        }

        slab_cache.empty = Some(NonNull::from_ref(active_slab));
    } else if was_full {
        let active_slab_lock_prev = active_slab_lock.prev;
        let active_slab_lock_next = active_slab_lock.next;

        active_slab_lock.prev = None;
        active_slab_lock.next = slab_cache.partial;
        drop(active_slab_lock);

        // Remove the newly partial slab from the full slab list.
        if let Some(prev) = active_slab_lock_prev {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `active_slab_lock.prev` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { prev.as_ref().inner.lock().next = active_slab_lock_next }
        } else {
            slab_cache.full = active_slab_lock_next;
        };

        if let Some(next) = active_slab_lock_next {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `active_slab_lock.prev` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next.as_ref().inner.lock().prev = active_slab_lock_prev }
        }

        // Add the new partial slab to the start of the partial slab list.
        if let Some(next_slab) = slab_cache.partial {
            // SAFETY:
            //
            // The [`SlabCache`] is locked so `empty.next` will not move during this
            // operation and initialized [`Slab`]s are always read utilizing immutable
            // references.
            unsafe { next_slab.as_ref().inner.lock().prev = Some(NonNull::from_ref(active_slab)) }
        }

        slab_cache.partial = Some(NonNull::from_ref(active_slab));
    }
}

fn calculate_cache_index(size: usize) -> usize {
    u32_to_usize(
        size.next_power_of_two()
            .ilog2()
            .saturating_sub(MIN_SIZE_CLASS.ilog2()),
    )
}

/// A cache for [`Slab`]s of a particular size.
struct SlabCache {
    object_size: usize,
    empty: Option<NonNull<Slab>>,
    partial: Option<NonNull<Slab>>,
    full: Option<NonNull<Slab>>,
}

// SAFETY:
//
// [`SlabCache`]s do not manipulate per-CPU or per-thead items.
unsafe impl Send for SlabCache {}

// SAFETY:
//
// [`SlabCache`] do not provide access to per-CPU or per-thead items.
unsafe impl Sync for SlabCache {}

/// A [`Page`][p]-sized subcomponent of the SLAB allocator subsystem.
///
/// [p]: crate::memory::virt::structs::Page
struct Slab {
    cache: &'static Spinlock<SlabCache>,
    frame: Frame,
    inner: Spinlock<SlabInner>,
}

struct SlabInner {
    free_list: *mut FreeObject,
    allocated: usize,
    prev: Option<NonNull<Slab>>,
    next: Option<NonNull<Slab>>,
}

impl Slab {
    pub fn alloc_slab(
        cache: &'static Spinlock<SlabCache>,
        object_size: usize,
    ) -> Option<NonNull<Slab>> {
        let count = usize_to_u64(SLAB_PAGE_FRAME_COUNT);
        let alignment = count.strict_mul(usize_to_u64(page_frame_size()));

        let frame_range = allocate_frames(count, alignment).ok()?;
        let page_range = map(frame_range, Permissions::ReadWrite).ok()?;

        let mut current = page_range
            .start()
            .start_address()
            .strict_add(mem::size_of::<Slab>())
            .strict_align_up(object_size);
        let end = page_range.end_address_exclusive();

        let mut head: *mut FreeObject = ptr::null_mut();
        while current < end {
            let obj = ptr::with_exposed_provenance_mut::<FreeObject>(current.value());

            // SAFETY:
            //
            // This [`Frame`] and [`Page`] have not escaped this function, which ensures that exclusive
            // access has been achieved.
            unsafe {
                (*obj).next = head;
            }
            head = obj;
            current = current.strict_add(object_size);
        }

        let slab = Slab {
            cache,
            frame: frame_range.start(),
            inner: Spinlock::new(SlabInner {
                free_list: head,
                allocated: 0,
                prev: None,
                next: None,
            }),
        };

        let slab_ptr =
            ptr::with_exposed_provenance_mut::<Slab>(page_range.start().start_address().value());
        // SAFETY:
        //
        // This [`Frame`] and [`Page`] have not escaped this function, which ensures that exclusive
        // access has been achieved.
        unsafe { slab_ptr.write(slab) };
        NonNull::new(slab_ptr)
    }
}

impl SlabInner {
    pub fn allocate(&mut self) -> NonNull<u8> {
        assert!(!self.full());

        let ptr = self.free_list;

        // SAFETY:
        //
        // The [`Slab`] is not full, which means that the free list points free objects.
        let next = unsafe { (*ptr).next };
        self.free_list = next;

        self.allocated += 1;
        NonNull::new(ptr.cast::<u8>()).expect("slab allocation should never be null")
    }

    // Deallocates the provided `object`.
    //
    // SAFETY:
    //
    // The provided `object` must have originally been allocated from this [`Slab`] and must be an
    // active allocation.
    pub unsafe fn deallocate(&mut self, object: *mut u8) {
        let object = object.cast::<FreeObject>();

        unsafe { (*object).next = self.free_list }
        self.free_list = object;
        self.allocated -= 1;
    }

    /// Returns `true` if the [`Slab`] has zero active allocation slots.
    pub fn empty(&self) -> bool {
        self.allocated == 0
    }

    /// Returns `true` if the [`Slab`] has some active allocation slots.
    pub fn partial(&self) -> bool {
        !self.empty() && !self.full()
    }

    /// Returns `true` if the [`Slab`] is full. This means that there are zero additional slots
    /// available for allocation.
    pub const fn full(&self) -> bool {
        self.free_list.is_null()
    }
}

struct FreeObject {
    next: *mut FreeObject,
}
