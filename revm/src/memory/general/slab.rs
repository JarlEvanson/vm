//! An fixed-size allocator based on Linux's SLAB allocator.

use core::{
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
    slice,
};

use sync::{ControlledModificationCell, RawSpinlock, Spinlock};

use crate::{
    memory::{
        page_frame_size,
        phys::{allocate_frames, structs::Frame},
        virt::{Permissions, map},
    },
    util::{u32_to_usize, usize_to_u32_panicking, usize_to_u64},
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
            object_size: min_size_class * 2usize.pow(usize_to_u32_panicking(index)),
            empty: ptr::null_mut(),
            partial: ptr::null_mut(),
            full: ptr::null_mut(),
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

    if !slab_cache.partial.is_null() {
        // Try to allocate from partial slabs first.

        let partial_slab_ptr = slab_cache.partial;
        let partial_slab = unsafe { &mut *partial_slab_ptr };

        let ptr = partial_slab.allocate();
        if partial_slab.full() {
            // Remove full slab from partial slab list.
            slab_cache.partial = partial_slab.next;

            // Add full slab start of full slab list.
            partial_slab.next = slab_cache.full;
            slab_cache.full = partial_slab_ptr;
        }

        Some(ptr)
    } else if !slab_cache.empty.is_null() {
        // Try to allocate from cached empty slabs.

        let empty_slab_ptr = slab_cache.empty;
        let empty_slab = unsafe { &mut *empty_slab_ptr };

        let ptr = empty_slab.allocate();

        // Remove new partial slab from empty slab list.
        slab_cache.empty = empty_slab.next;

        // Add new partial slab to the start of the partial slab list.
        empty_slab.next = slab_cache.partial;
        slab_cache.partial = empty_slab_ptr;

        Some(ptr)
    } else {
        // Otherwise, we must allocate a new [`Slab`].

        let slab_ptr = Slab::alloc_slab(slab_cache_ref, slab_cache.object_size)?;

        let slab = unsafe { &mut *slab_ptr };
        let ptr = slab.allocate();

        slab.next = slab_cache.partial;
        slab_cache.partial = slab_ptr;

        Some(ptr)
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
    empty: *mut Slab,
    partial: *mut Slab,
    full: *mut Slab,
}

// SAFETY:
//
// [`SlabCache`]s do not manipulate per-CPU or per-thead items.
unsafe impl Send for SlabCache {}

// SAFETY:
//
// [`SlabCache`] do not provide access to per-CPU or per-thead items.
unsafe impl Sync for SlabCache {}

/// A [`Page`]-sized subcomponent of the SLAB allocator subsystem.
struct Slab {
    cache: &'static Spinlock<SlabCache>,
    frame: Frame,
    free_list: *mut FreeObject,
    allocated: usize,
    next: *mut Slab,
}

impl Slab {
    pub fn alloc_slab(
        cache: &'static Spinlock<SlabCache>,
        object_size: usize,
    ) -> Option<*mut Slab> {
        let count = usize_to_u64(SLAB_PAGE_FRAME_COUNT);
        let alignment = count.strict_mul(usize_to_u64(page_frame_size()));

        let frame_range = allocate_frames(count, alignment).ok()?;
        let page_range = map(frame_range, Permissions::ReadWrite).ok()?;

        let mut current = page_range
            .start()
            .start_address()
            .add(mem::size_of::<Slab>())
            .align_up(object_size);
        let end = page_range.end().end_address();

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
            current = current.add(object_size);
        }

        let slab = Slab {
            cache,
            frame: frame_range.start(),
            free_list: head,
            allocated: 0,
            next: ptr::null_mut(),
        };

        let slab_ptr =
            ptr::with_exposed_provenance_mut::<Slab>(page_range.start().start_address().value());
        // SAFETY:
        //
        // This [`Frame`] and [`Page`] have not escaped this function, which ensures that exclusive
        // access has been achieved.
        unsafe { slab_ptr.write(slab) };
        Some(slab_ptr)
    }

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
