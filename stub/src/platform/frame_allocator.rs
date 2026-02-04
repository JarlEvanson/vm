//! Implementation of a generic memory map tracker that provides [`FrameRange`] allocation from
//! said memory map.

use core::{
    hash::{BuildHasher, BuildHasherDefault, Hash, Hasher},
    mem::{self, offset_of},
};

use stub_api::{MemoryDescriptor, MemoryType};
use sync::Spinlock;

use crate::{
    platform::{
        AllocationPolicy, BufferTooSmall, Frame, FrameRange, MemoryMap, OutOfMemory,
        PhysicalAddress, frame_size, read_u32_at, read_u64_at, write_u32_at, write_u64_at,
    },
    util::{u64_to_usize_panicking, usize_to_u64},
};

/// The link value indicating that the linked list has ended.
const END_LINK: PhysicalAddress = PhysicalAddress::zero().add(1);
/// The size, in bytes, of [`MemoryDescriptor`]s.
const DESCRIPTOR_SIZE: u64 = usize_to_u64(mem::size_of::<MemoryDescriptor>());

/// The [`FrameAllocator`] for use on the entire system.
static FRAME_ALLOCATOR: Spinlock<FrameAllocator> = Spinlock::new(FrameAllocator::new());

/// Initializes the system [`FrameAllocator`].
pub fn initialize<I: Iterator<Item = MemoryDescriptor> + Clone + ExactSizeIterator>(iter: I) {
    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    let space_inside_link = frame_size().strict_sub(usize_to_u64(mem::size_of::<u64>()));
    frame_allocator.descriptors_per_link = space_inside_link / DESCRIPTOR_SIZE;

    // Insert regions of free memory.
    for descriptor in iter
        .clone()
        .filter(|descriptor| descriptor.region_type == MemoryType::FREE)
    {
        crate::trace!("inserting {descriptor:x?}");
        let Err(()) = frame_allocator.try_insert_region(descriptor) else {
            frame_allocator.validate_order();
            continue;
        };

        let link = frame_allocator.find_link_init(iter.clone()).unwrap();
        crate::trace!("stealing link {link:x?}");
        frame_allocator.add_link(link);

        assert!(frame_allocator.try_insert_region(descriptor).is_ok());
        frame_allocator.validate_order();
    }

    for descriptor in iter
        .clone()
        .filter(|descriptor| descriptor.region_type != MemoryType::FREE)
    {
        crate::trace!("inserting {descriptor:x?}");
        let Err(()) = frame_allocator.try_insert_region(descriptor) else {
            frame_allocator.validate_order();
            continue;
        };

        let link = frame_allocator.find_link_init(iter.clone()).unwrap();
        crate::trace!("stealing link {link:x?}");
        frame_allocator.add_link(link);

        assert!(frame_allocator.try_insert_region(descriptor).is_ok());
        frame_allocator.validate_order();
    }

    'link_loop: loop {
        let mut link = frame_allocator.link;
        while link != END_LINK {
            let descriptor = MemoryDescriptor {
                number: Frame::containing_address(link).number(),
                count: 1,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };

            crate::trace!("inserting {descriptor:x?}");
            let Err(()) = frame_allocator.try_insert_region(descriptor) else {
                link = PhysicalAddress::new(read_u64_at(link));
                continue;
            };

            let link = frame_allocator.find_link_init(iter.clone()).unwrap();
            frame_allocator.add_link(link);
            continue 'link_loop;
        }

        break;
    }

    frame_allocator.validate();

    crate::debug!("Initial Memory Map:");
    for descriptors in frame_allocator.descriptors() {
        crate::debug!("{descriptors:x?}");
    }
}

/// Implementation of [`crate::platform::allocate_frames()`] using the system [`FrameAllocator`].
pub fn allocate_frames(count: u64, policy: AllocationPolicy) -> Result<FrameRange, OutOfMemory> {
    let mut allocator = FRAME_ALLOCATOR.lock();

    macro_rules! base_iter {
        () => {
            allocator
                .descriptors()
                .filter(|descriptor| descriptor.region_type == MemoryType::FREE)
                .map(|descriptor| {
                    if descriptor.number == 0 {
                        MemoryDescriptor {
                            number: 1,
                            count: descriptor.count - 1,
                            region_type: descriptor.region_type,
                        }
                    } else {
                        descriptor
                    }
                })
        };
    }

    let result = match policy {
        AllocationPolicy::Any => loop {
            let descriptor = base_iter!()
                .find(|descriptor| descriptor.count >= count)
                .ok_or(OutOfMemory)?;

            let descriptor_range = frame_range_from_descriptor(descriptor);
            let descriptor = MemoryDescriptor {
                number: descriptor_range.start().number(),
                count,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            match allocator.try_insert_region(descriptor) {
                Ok(()) => break Ok(FrameRange::new(descriptor_range.start(), count)),
                Err(()) => allocator.allocate_link(),
            }
        },
        AllocationPolicy::At(address) => loop {
            assert!(address.is_multiple_of(frame_size()));
            let range = FrameRange::new(
                Frame::containing_address(PhysicalAddress::new(address)),
                count,
            );

            let _ = base_iter!()
                .find(|&descriptor| {
                    let descriptor_range = frame_range_from_descriptor(descriptor);

                    range.intersection(descriptor_range) == range
                })
                .ok_or(OutOfMemory)?;

            let descriptor = MemoryDescriptor {
                number: range.start().number(),
                count,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            match allocator.try_insert_region(descriptor) {
                Ok(()) => break Ok(range),
                Err(()) => allocator.allocate_link(),
            }
        },
        AllocationPolicy::Below(address) => loop {
            let descriptor = base_iter!()
                .filter(|&descriptor| {
                    let descriptor_range = frame_range_from_descriptor(descriptor);

                    descriptor_range.end().end_address().value() <= address
                })
                .find(|descriptor| descriptor.count >= count)
                .ok_or(OutOfMemory)?;

            let descriptor_range = frame_range_from_descriptor(descriptor);
            let descriptor = MemoryDescriptor {
                number: descriptor_range.start().number(),
                count,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            match allocator.try_insert_region(descriptor) {
                Ok(()) => break Ok(FrameRange::new(descriptor_range.start(), count)),
                Err(()) => allocator.allocate_link(),
            }
        },
    };

    allocator.validate();
    result
}

/// Implementation of [`crate::platform::deallocate_frames()`] using the system [`FrameAllocator`].
pub unsafe fn deallocate_frames(range: FrameRange) {
    let mut allocator = FRAME_ALLOCATOR.lock();
    loop {
        let descriptor = MemoryDescriptor {
            number: range.start().number(),
            count: range.count(),
            region_type: MemoryType::FREE,
        };
        match allocator.try_insert_region(descriptor) {
            Ok(()) => break,
            Err(()) => allocator.allocate_link(),
        }
    }

    allocator.validate();
}

/// Implementation of [`crate::platform::memory_map()`] using the system [`FrameAllocator`].
pub fn memory_map<'buffer>(
    buffer: &'buffer mut [MemoryDescriptor],
) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
    let frame_allocator = FRAME_ALLOCATOR.lock();

    if usize_to_u64(buffer.len()) < frame_allocator.range_count {
        return Err(BufferTooSmall {
            required_count: u64_to_usize_panicking(frame_allocator.range_count),
        });
    }

    for (write_loc, descriptor) in buffer.iter_mut().zip(frame_allocator.descriptors()) {
        *write_loc = descriptor;
    }

    let mut state = BuildHasherDefault::<Fnv1aHash>::new().build_hasher();
    buffer[..u64_to_usize_panicking(frame_allocator.range_count)]
        .iter()
        .for_each(|descriptor| {
            descriptor.number.hash(&mut state);
            descriptor.count.hash(&mut state);
            descriptor.region_type.0.hash(&mut state);
        });

    Ok(MemoryMap::new(buffer, state.finish()))
}

/// Frame allocator and memory map tracker.
struct FrameAllocator {
    /// The physical address of the linked list frames containing [`MemoryDescriptor`]s.
    link: PhysicalAddress,
    /// The total number of ranges.
    range_count: u64,
    /// The total number of links.
    link_count: u64,

    /// The number of descriptors stored in each link.
    descriptors_per_link: u64,
}

impl FrameAllocator {
    /// Creates an empty [`FrameAllocator`].
    pub const fn new() -> Self {
        Self {
            link: END_LINK,
            range_count: 0,
            link_count: 0,
            descriptors_per_link: 0,
        }
    }

    /// Attempts to insert a [`MemoryDescriptor`] into the [`FrameAllocator`]'s tracking.
    fn try_insert_region(&mut self, mut descriptor: MemoryDescriptor) -> Result<(), ()> {
        let mut range = frame_range_from_descriptor(descriptor);

        // Initialize the current link.
        let mut current_link = self.link;

        let mut lower_overlap_location = None;
        let mut subsuming_overlap_location = None;
        let mut subsuming_overlap_count = 0;
        let mut upper_overlap_location = None;

        let mut current_index = 0;
        'link_loop: while current_link != END_LINK {
            let mut sublink_index = 0;
            while sublink_index < self.descriptors_per_link {
                if current_index >= self.range_count {
                    // All descriptors have been processed.
                    break 'link_loop;
                }

                let sublink_descriptor = read_descriptor(current_link, sublink_index);
                let sublink_range = frame_range_from_descriptor(sublink_descriptor);

                // The entirety of the sublink descriptor is before the target descriptor and thus
                // the sublink descriptor cannot overlap with the target descriptor.
                //
                // Since the descriptors are ordered and do not overlap within the list, we can
                // skip forward until overlapping or adjacency begins.
                if sublink_range.end() < range.start() {
                    sublink_index += 1;
                    current_index += 1;
                    continue;
                }

                // The entirety of the sublink descriptor is after the target descriptor and thus the
                // sublink descriptor cannot overlap with the target descriptor.
                //
                // Since the descriptors are ordered and do not overlap within the list, all
                // possible overlaps have been processed.
                if sublink_range.start() > range.end() {
                    break 'link_loop;
                }

                // Any sublink descriptors that reach this point must be adjacent to or overlap
                // with the target descriptor.

                // Any overlapping or adjacent regions that share [`MemoryType`]s with the target
                // descriptor are merged.
                if sublink_descriptor.region_type == descriptor.region_type {
                    subsuming_overlap_location =
                        subsuming_overlap_location.or(Some((current_link, sublink_index)));
                    subsuming_overlap_count += 1;

                    range = range
                        .merge(sublink_range)
                        .expect("adjacent regions failed to merge");

                    sublink_index += 1;
                    current_index += 1;
                    continue;
                }

                let (lower, overlap, upper) = range.partition_with(sublink_range);

                // Check for adjacent regions (adjacent regions of differing types don't matter and
                // that is all that will reach this point since the previous check eliminated same
                // type regions).
                if overlap.is_empty() {
                    if sublink_range.start() >= range.end() {
                        break 'link_loop;
                    }

                    sublink_index += 1;
                    current_index += 1;
                    continue;
                }

                match (lower.is_empty(), upper.is_empty()) {
                    (true, true) => {
                        // The entire [`MemoryDescriptor`] is subsumed by `descriptor`.
                        subsuming_overlap_location =
                            subsuming_overlap_location.or(Some((current_link, sublink_index)));
                        subsuming_overlap_count += 1;
                    }
                    (true, false) => upper_overlap_location = Some((current_link, sublink_index)),
                    (false, true) => lower_overlap_location = Some((current_link, sublink_index)),
                    (false, false) => {
                        if self.range_count.strict_add(2) > self.current_capacity() {
                            return Err(());
                        }

                        let lower_descriptor = MemoryDescriptor {
                            number: lower.start().number(),
                            count: lower.count(),
                            region_type: sublink_descriptor.region_type,
                        };
                        write_descriptor(current_link, sublink_index, lower_descriptor);

                        self.next_location(&mut current_link, &mut sublink_index);
                        self.shift_one_up(current_link, sublink_index);

                        write_descriptor(current_link, sublink_index, descriptor);

                        self.next_location(&mut current_link, &mut sublink_index);
                        self.shift_one_up(current_link, sublink_index);

                        let upper_descriptor = MemoryDescriptor {
                            number: upper.start().number(),
                            count: upper.count(),
                            region_type: sublink_descriptor.region_type,
                        };
                        write_descriptor(current_link, sublink_index, upper_descriptor);

                        self.range_count += 2;
                        return Ok(());
                    }
                }

                sublink_index += 1;
                current_index += 1;
            }

            current_link = PhysicalAddress::new(read_u64_at(current_link));
        }

        if let Some((link, index)) = lower_overlap_location {
            let mut lower_descriptor = read_descriptor(link, index);

            lower_descriptor.count = range.start().number() - lower_descriptor.number;
            write_descriptor(link, index, lower_descriptor);
        }

        if let Some((link, index)) = upper_overlap_location {
            let mut upper_descriptor = read_descriptor(link, index);
            let upper_range = frame_range_from_descriptor(upper_descriptor);

            upper_descriptor.number = range.end().number();
            upper_descriptor.count = upper_range.end().number() - range.end().number();
            write_descriptor(link, index, upper_descriptor);
        }

        if let Some((link, index)) = subsuming_overlap_location {
            descriptor.number = range.start().number();
            descriptor.count = range.count();
            write_descriptor(link, index, descriptor);

            let mut write_link = link;
            let mut write_index = index;
            self.next_location(&mut write_link, &mut write_index);

            let mut read_link = link;
            let mut read_index = index;
            for _ in 0..subsuming_overlap_count {
                self.next_location(&mut read_link, &mut read_index);
            }

            while read_link != END_LINK {
                let descriptor = read_descriptor(read_link, read_index);
                write_descriptor(write_link, write_index, descriptor);

                self.next_location(&mut write_link, &mut write_index);
                self.next_location(&mut read_link, &mut read_index);
            }

            self.range_count -= subsuming_overlap_count - 1;
        } else {
            // Shift descriptors up 1.
            if self.range_count.strict_add(1) > self.current_capacity() {
                return Err(());
            }

            let (link, index) = if let Some(loc) = upper_overlap_location {
                loc
            } else {
                (current_link, current_index % self.descriptors_per_link)
            };
            self.shift_one_up(link, index);

            let descriptor = MemoryDescriptor {
                number: range.start().number(),
                count: range.count(),
                region_type: descriptor.region_type,
            };
            write_descriptor(link, index, descriptor);

            self.range_count += 1;
        }

        Ok(())
    }

    /// Shifts stored [`MemoryDescriptor`]s up a single time.
    fn shift_one_up(&mut self, link: PhysicalAddress, index: u64) {
        let mut carry_link = link;
        let mut carry_index = index;

        let mut carry = {
            let last = read_descriptor(link, self.descriptors_per_link - 1);
            for index in (carry_index..(self.descriptors_per_link - 1)).rev() {
                let descriptor = read_descriptor(carry_link, index);
                write_descriptor(carry_link, index + 1, descriptor);
            }

            last
        };

        loop {
            // Force move to next link.
            carry_index = self.descriptors_per_link;
            self.next_location(&mut carry_link, &mut carry_index);
            if carry_link == END_LINK {
                break;
            }

            // Save last [`MemoryDescriptor`] in the link.
            let storage = read_descriptor(carry_link, self.descriptors_per_link - 1);

            // Copy each descriptor up a single index.
            for index in (0..(self.descriptors_per_link - 1)).rev() {
                let descriptor = read_descriptor(carry_link, index);
                write_descriptor(carry_link, index + 1, descriptor);
            }

            // Write the carried descriptor into the first entry of the link.
            write_descriptor(carry_link, 0, carry);

            carry = storage;
        }
    }

    /// Allocates a single link in the [`FrameAllocator`]'s linked list.
    fn allocate_link(&mut self) {
        let mut link = self.link;
        let mut index = 0;
        let mut current_index = 0;
        while link != END_LINK && current_index <= self.range_count {
            let mut descriptor = read_descriptor(link, index);

            // Skip the descriptor if it isn't free memory or if it is the zero frame.
            if descriptor.region_type != MemoryType::FREE || descriptor.number == 0 {
                self.next_location(&mut link, &mut index);
                current_index += 1;
                continue;
            }

            // Steal the first frame for use as a link.
            let range = frame_range_from_descriptor(descriptor);
            self.add_link(range.start().start_address());

            descriptor.count = 1;
            descriptor.region_type = MemoryType::BOOTLOADER_RECLAIMABLE;
            self.try_insert_region(descriptor)
                .expect("link failed to add additonal storage");

            self.validate();
            return;
        }
    }

    /// Adds an allocated link to the [`FrameAllocator`]'s linked list.
    #[track_caller]
    fn add_link(&mut self, link: PhysicalAddress) {
        let mut previous_link = END_LINK;
        let mut current_link = self.link;
        while current_link != END_LINK {
            previous_link = current_link;
            current_link = PhysicalAddress::new(read_u64_at(current_link));
        }

        if previous_link == END_LINK {
            self.link = link;
            write_u64_at(self.link, END_LINK.value());
        } else {
            write_u64_at(previous_link, link.value());
            write_u64_at(link, END_LINK.value());
        }

        self.link_count += 1;
    }

    /// Determines the location of a single link in the [`FrameAllocator`]'s linked list during the
    /// [`initialize()`] call.
    fn find_link_init<I: Iterator<Item = MemoryDescriptor> + Clone + ExactSizeIterator>(
        &self,
        iter: I,
    ) -> Option<PhysicalAddress> {
        for descriptor in iter.clone() {
            assert_ne!(descriptor.count, 0);
            if descriptor.region_type != MemoryType::FREE {
                continue;
            }

            let descriptor_range = frame_range_from_descriptor(descriptor);
            'frame_finder: for frame in descriptor_range.iter() {
                // Skip the zero frame - it has complexities that this [`FrameAllocator`] doesn't
                // want to deal with.
                if frame == Frame::zero() {
                    continue;
                }

                // Validate that the frame does not overlap with any non-free regions.
                for check_descriptor in iter.clone() {
                    if check_descriptor.region_type == MemoryType::FREE {
                        continue;
                    }

                    let check_range = frame_range_from_descriptor(check_descriptor);
                    if check_range.contains(frame) {
                        continue 'frame_finder;
                    }
                }

                // Validate the the frame is actually free (namely, that it is not already being
                // used as a link for the [`FrameAllocator`]).
                for link in self.links() {
                    if Frame::containing_address(link) == frame {
                        continue 'frame_finder;
                    }
                }

                return Some(frame.start_address());
            }
        }

        None
    }

    /// Returns an iterator over the links in the [`FrameAllocator`]'s linked list.
    pub fn links(&self) -> LinkIter<'_> {
        LinkIter {
            _allocator: self,
            current_link: self.link,
        }
    }

    /// Returns an iterator over the [`MemoryDescriptor`]s stored in the [`FrameAllocator`]'s
    /// linked list.
    pub fn descriptors(&self) -> DescriptorIter<'_> {
        DescriptorIter {
            allocator: self,
            link: self.link,
            index: 0,
            total_index: 0,
        }
    }

    /// Given a `link` and a sublink `index`, this updates them to their next location.
    pub fn next_location(&self, link: &mut PhysicalAddress, index: &mut u64) {
        if *link == END_LINK {
            return;
        }

        if *index >= self.descriptors_per_link {
            *link = PhysicalAddress::new(read_u64_at(*link));
            *index = 0;
            return;
        }

        *index += 1;
    }

    /// Returns the number of [`MemoryDescriptor`]s that the [`FrameAllocator`] can store without
    /// allocating another link.
    pub const fn current_capacity(&self) -> u64 {
        self.link_count.strict_mul(self.descriptors_per_link)
    }

    /// Validates the ordering invariant of this [`FrameAllocator`].
    #[track_caller]
    pub fn validate_order(&self) {
        let mut empty_descriptor = false;
        let mut out_of_order_descriptor = false;

        let mut current_end = Frame::zero();
        for descriptor in self.descriptors().map(frame_range_from_descriptor) {
            empty_descriptor = empty_descriptor || descriptor.is_empty();
            out_of_order_descriptor = out_of_order_descriptor || current_end > descriptor.start();

            current_end = descriptor.end();
        }

        if empty_descriptor || out_of_order_descriptor {
            for descriptor in self.descriptors() {
                crate::error!(
                    "{:x?} {:?}",
                    frame_range_from_descriptor(descriptor),
                    descriptor.region_type
                );
            }

            assert!(
                !empty_descriptor,
                "empty memory descriptor in FrameAllocator"
            );
            assert!(
                !out_of_order_descriptor,
                "out of order memory descriptor in FrameAllocator"
            );
        }
    }

    /// Validates the invariants of this [`FrameAllocator`].
    #[track_caller]
    pub fn validate(&self) {
        self.validate_order();

        let mut link_in_free_mem = false;
        for link in self.links() {
            let link_frame = Frame::containing_address(link);

            for descriptor in self.descriptors() {
                let range = frame_range_from_descriptor(descriptor);
                if descriptor.region_type != MemoryType::FREE {
                    continue;
                }

                link_in_free_mem = link_in_free_mem || range.contains(link_frame);
            }
        }

        if link_in_free_mem {
            for descriptor in self.descriptors() {
                crate::error!(
                    "{:x?} {:?}",
                    frame_range_from_descriptor(descriptor),
                    descriptor.region_type
                );
            }

            for link in self.links() {
                crate::error!("Link: {link:x?}");
            }

            assert!(!link_in_free_mem, "link frame is marked as free");
        }
    }
}

/// An [`Iterator`] over the links of a [`FrameAllocator`].
struct LinkIter<'allocator> {
    /// The [`FrameAllocator`], which is held to ensure proper lifetimes.
    _allocator: &'allocator FrameAllocator,
    /// The current link.
    current_link: PhysicalAddress,
}

impl Iterator for LinkIter<'_> {
    type Item = PhysicalAddress;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_link == END_LINK {
            return None;
        }

        let value = self.current_link;
        self.current_link = PhysicalAddress::new(read_u64_at(self.current_link));
        Some(value)
    }
}

/// An [`Iterator`] over the [`MemoryDescriptor`]s of a [`FrameAllocator`].
struct DescriptorIter<'allocator> {
    /// The [`FrameAllocator`].
    allocator: &'allocator FrameAllocator,

    /// The current link.
    link: PhysicalAddress,
    /// The index into the [`MemoryDescriptor`]s in the current link.
    index: u64,
    /// The overall index into the [`MemoryDescriptor`]s.
    ///
    /// This is the sum of all indices this function has enumerated.
    total_index: u64,
}

impl Iterator for DescriptorIter<'_> {
    type Item = MemoryDescriptor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.link == END_LINK || self.total_index >= self.allocator.range_count {
            return None;
        }

        let value = read_descriptor(self.link, self.index);
        self.allocator
            .next_location(&mut self.link, &mut self.index);
        self.total_index += 1;
        Some(value)
    }
}

/// Reads a [`MemoryDescriptor`] given its link and index in the link's storage.
fn read_descriptor(link: PhysicalAddress, index: u64) -> MemoryDescriptor {
    let array_offset = link.add(usize_to_u64(mem::size_of::<u64>()));
    let entry_offset = index.strict_mul(DESCRIPTOR_SIZE);
    read_descriptor_at(array_offset.add(entry_offset))
}

/// Writes a [`MemoryDescriptor`] given the target link and index.
fn write_descriptor(link: PhysicalAddress, index: u64, descriptor: MemoryDescriptor) {
    let array_offset = link.add(usize_to_u64(mem::size_of::<u64>()));
    let entry_offset = index.strict_mul(DESCRIPTOR_SIZE);
    write_descriptor_at(array_offset.add(entry_offset), descriptor)
}

/// Reads a [`MemoryDescriptor`] from the given physical memory.
fn read_descriptor_at(physical_address: PhysicalAddress) -> MemoryDescriptor {
    let number_offset = usize_to_u64(offset_of!(MemoryDescriptor, number));
    let count_offset = usize_to_u64(offset_of!(MemoryDescriptor, count));
    let region_type_offset = usize_to_u64(offset_of!(MemoryDescriptor, region_type));

    let number = read_u64_at(physical_address.add(number_offset));
    let count = read_u64_at(physical_address.add(count_offset));
    let region_type = MemoryType(read_u32_at(physical_address.add(region_type_offset)));

    MemoryDescriptor {
        number,
        count,
        region_type,
    }
}

/// Writes a [`MemoryDescriptor`] into the given physical memory.
fn write_descriptor_at(physical_address: PhysicalAddress, descriptor: MemoryDescriptor) {
    let number_offset = usize_to_u64(offset_of!(MemoryDescriptor, number));
    let count_offset = usize_to_u64(offset_of!(MemoryDescriptor, count));
    let region_type_offset = usize_to_u64(offset_of!(MemoryDescriptor, region_type));

    write_u64_at(physical_address.add(number_offset), descriptor.number);
    write_u64_at(physical_address.add(count_offset), descriptor.count);
    write_u32_at(
        physical_address.add(region_type_offset),
        descriptor.region_type.0,
    );
}

/// Returns the [`FrameRange`] the [`MemoryDescriptor`] describes.
fn frame_range_from_descriptor(descriptor: MemoryDescriptor) -> FrameRange {
    FrameRange::new(Frame::new(descriptor.number), descriptor.count)
}

/// [`Hasher`] implementing `FNV-1a`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Fnv1aHash(u64);

impl Hasher for Fnv1aHash {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for Fnv1aHash {
    fn default() -> Self {
        Self(0xcbf29ce484222325)
    }
}
