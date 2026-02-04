//! Implementation of a generic memory map frame allocator.

use core::{
    hash::{BuildHasher, BuildHasherDefault, Hash, Hasher},
    mem::{self, offset_of},
};

use stub_api::{MemoryDescriptor, MemoryType};
use sync::Spinlock;

use crate::{
    platform::{
        AllocationPolicy, BufferTooSmall, MemoryMap, OutOfMemory, frame_size, read_u32_at,
        read_u64_at, write_u32_at, write_u64_at,
    },
    util::{u64_to_usize, usize_to_u64},
};

/// Value indicating that the linked list has ended.
const END_LINK: u64 = 0b1;
/// The size, in bytes, of [`MemoryDescriptor`]s.
const DESCRIPTOR_SIZE: u64 = usize_to_u64(mem::size_of::<MemoryDescriptor>());

static FRAME_ALLOCATOR: Spinlock<FrameAllocator> = Spinlock::new(FrameAllocator {
    link: END_LINK,
    range_count: 0,
    link_count: 0,
    descriptors_per_link: 0,
});

pub fn initialize<I: Iterator<Item = MemoryDescriptor> + Clone + ExactSizeIterator>(iter: I) {
    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    let space_inside_link = frame_size().strict_sub(usize_to_u64(mem::size_of::<u64>()));
    frame_allocator.descriptors_per_link = space_inside_link / DESCRIPTOR_SIZE;

    // Insert regions of free memory.
    for descriptor in iter
        .clone()
        .filter(|descriptor| descriptor.region_type == MemoryType::FREE)
    {
        let Err(()) = frame_allocator.try_insert_region(descriptor) else {
            continue;
        };

        let link = frame_allocator.find_link_init(iter.clone()).unwrap();
        frame_allocator.add_link(link);

        assert!(frame_allocator.try_insert_region(descriptor).is_ok());
    }

    for descriptor in iter
        .clone()
        .filter(|descriptor| descriptor.region_type != MemoryType::FREE)
    {
        let Err(()) = frame_allocator.try_insert_region(descriptor) else {
            continue;
        };

        let link = frame_allocator.find_link_init(iter.clone()).unwrap();
        frame_allocator.add_link(link);

        assert!(frame_allocator.try_insert_region(descriptor).is_ok());
    }

    'link_loop: loop {
        let mut link = frame_allocator.link;
        while link != END_LINK {
            let descriptor = MemoryDescriptor {
                start: link,
                count: 1,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            let Err(()) = frame_allocator.try_insert_region(descriptor) else {
                link = read_u64_at(link);
                continue;
            };

            let link = frame_allocator.find_link_init(iter.clone()).unwrap();
            frame_allocator.add_link(link);
            continue 'link_loop;
        }

        break;
    }
}

pub fn allocate_frames(count: u64, policy: AllocationPolicy) -> Result<u64, OutOfMemory> {
    let mut allocator = FRAME_ALLOCATOR.lock();

    macro_rules! base_iter {
        () => {
            allocator
                .descriptors()
                .filter(|descriptor| descriptor.region_type == MemoryType::FREE)
                .map(|descriptor| {
                    if descriptor.start == 0 {
                        MemoryDescriptor {
                            start: frame_size(),
                            count: descriptor.count - 1,
                            region_type: descriptor.region_type,
                        }
                    } else {
                        descriptor
                    }
                })
        };
    }

    match policy {
        AllocationPolicy::Any => loop {
            let descriptor = base_iter!()
                .find(|descriptor| descriptor.count >= count)
                .ok_or(OutOfMemory)?;

            let physical_base = descriptor.start;

            let descriptor = MemoryDescriptor {
                start: descriptor.start,
                count,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            match allocator.try_insert_region(descriptor) {
                Ok(()) => break Ok(physical_base),
                Err(()) => allocator.allocate_link(),
            }
        },
        AllocationPolicy::At(address) => loop {
            let _ = base_iter!()
                .find(|descriptor| {
                    let end = descriptor
                        .start
                        .strict_add(descriptor.count.strict_mul(frame_size()));
                    let required_end = address.strict_add(count.strict_mul(frame_size()));
                    descriptor.start <= address && required_end <= end
                })
                .ok_or(OutOfMemory)?;

            let descriptor = MemoryDescriptor {
                start: address,
                count,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            match allocator.try_insert_region(descriptor) {
                Ok(()) => break Ok(address),
                Err(()) => allocator.allocate_link(),
            }
        },
        AllocationPolicy::Below(address) => loop {
            let descriptor = base_iter!()
                .filter(|descriptor| {
                    let end = descriptor.start.strict_add(count.strict_mul(frame_size()));
                    end < address
                })
                .find(|descriptor| descriptor.count >= count)
                .ok_or(OutOfMemory)?;

            let physical_base = descriptor.start;

            let descriptor = MemoryDescriptor {
                start: descriptor.start,
                count,
                region_type: MemoryType::BOOTLOADER_RECLAIMABLE,
            };
            match allocator.try_insert_region(descriptor) {
                Ok(()) => break Ok(physical_base),
                Err(()) => allocator.allocate_link(),
            }
        },
    }
}

pub unsafe fn deallocate_frames(physical_address: u64, count: u64) {
    let mut allocator = FRAME_ALLOCATOR.lock();
    loop {
        let descriptor = MemoryDescriptor {
            start: physical_address,
            count,
            region_type: MemoryType::FREE,
        };
        match allocator.try_insert_region(descriptor) {
            Ok(()) => break,
            Err(()) => allocator.allocate_link(),
        }
    }
}

pub fn memory_map<'buffer>(
    buffer: &'buffer mut [MemoryDescriptor],
) -> Result<MemoryMap<'buffer>, BufferTooSmall> {
    let frame_allocator = FRAME_ALLOCATOR.lock();

    if usize_to_u64(buffer.len()) < frame_allocator.range_count {
        return Err(BufferTooSmall {
            required_count: u64_to_usize(frame_allocator.range_count),
        });
    }

    for (write_loc, descriptor) in buffer.iter_mut().zip(frame_allocator.descriptors()) {
        *write_loc = descriptor;
    }

    let mut state = BuildHasherDefault::<Fnv1aHash>::new().build_hasher();
    buffer[..u64_to_usize(frame_allocator.range_count)]
        .iter()
        .for_each(|descriptor| {
            descriptor.start.hash(&mut state);
            descriptor.count.hash(&mut state);
            descriptor.region_type.0.hash(&mut state);
        });

    Ok(MemoryMap::new(buffer, state.finish()))
}

struct FrameAllocator {
    /// The physical address of the linked list frames containing [`MemoryDescriptor`]s.
    link: u64,
    /// The total number of ranges.
    range_count: u64,
    /// The total number of links.
    link_count: u64,

    /// The number of descriptors stored in each link.
    descriptors_per_link: u64,
}

impl FrameAllocator {
    fn try_insert_region(&mut self, descriptor: MemoryDescriptor) -> Result<(), ()> {
        // crate::trace!("attempting to insert {descriptor:x?}");

        // Calculate boundaries of the descriptor.
        let mut current_start = descriptor.start;
        let mut current_end = current_start.strict_add(descriptor.count.strict_mul(frame_size()));

        // Initialize the current link.
        let mut current_link = self.link;

        let mut lower_overlap_location = None;
        let mut subsuming_overlap_location = None;
        let mut subsuming_overlap_count = 0;
        let mut upper_overlap_location = None;

        let mut current_index = 0;
        'link_loop: while current_link != END_LINK {
            // crate::trace!("processing link at {current_link:#x}");

            let mut sublink_index = 0;
            while sublink_index < self.descriptors_per_link {
                if current_index >= self.range_count {
                    // All descriptors have been processed.
                    break 'link_loop;
                }

                let mut sublink_descriptor = read_descriptor(current_link, sublink_index);
                // crate::trace!("checking against {sublink_descriptor:x?}");

                // Calculate boundaries of the descriptor to check against.
                let sublink_start = sublink_descriptor.start;
                let sublink_end =
                    sublink_start.strict_add(sublink_descriptor.count.strict_mul(frame_size()));

                // The entirety of the sublink descriptor is before the target descriptor and thus
                // the sublink descriptor cannot overlap with the target descriptor.
                //
                // Since the descriptors are ordered and do not overlap within the list, we can
                // skip forward until overlapping or adjacency begins.
                if sublink_end < current_start {
                    sublink_index += 1;
                    current_index += 1;
                    continue;
                }

                // The entirety of the sublink descriptor is after the target descriptor and thus the
                // sublink descriptor cannot overlap with the target descriptor.
                //
                // Since the descriptors are ordered and do not overlap within the list, all
                // possible overlaps have been processed.
                if sublink_start > current_end {
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

                    current_start = current_start.min(sublink_start);
                    current_end = current_end.max(sublink_end);

                    sublink_index += 1;
                    current_index += 1;
                    continue;
                }

                // Calculate boundaries of the overlapping descriptors.
                let overlap_start = current_start.max(sublink_start);
                let overlap_end = current_end.min(sublink_end);

                // Check for adjacent regions (adjacent regions of differing types don't matter).
                let overlap_size = overlap_end - overlap_start;
                let overlap_count = overlap_size / frame_size();
                if overlap_count == 0 {
                    if sublink_start >= current_end {
                        break 'link_loop;
                    }

                    sublink_index += 1;
                    current_index += 1;
                    continue;
                }

                // The first condition checks if there remains a region on the lower end while the
                // second checks whether there remains a region on the upper end.
                match (overlap_start > sublink_start, overlap_end < sublink_end) {
                    (true, true) => {
                        // The target descriptor splits the sublink descriptor into two
                        // additional descriptors.

                        if self.range_count.strict_add(2) > self.max_capacity() {
                            return Err(());
                        }

                        sublink_descriptor.count = (overlap_start - sublink_start) / frame_size();
                        write_descriptor(current_link, sublink_index, sublink_descriptor);

                        self.next_location(&mut current_link, &mut sublink_index);
                        self.shift_one_up(current_link, current_index);
                        self.shift_one_up(current_link, current_index);

                        let descriptor = MemoryDescriptor {
                            start: current_start,
                            count: (current_end - current_start) / frame_size(),
                            region_type: descriptor.region_type,
                        };
                        write_descriptor(current_link, sublink_index, descriptor);

                        self.next_location(&mut current_link, &mut sublink_index);
                        let descriptor = MemoryDescriptor {
                            start: overlap_end,
                            count: (sublink_end - overlap_end) / frame_size(),
                            region_type: sublink_descriptor.region_type,
                        };
                        write_descriptor(current_link, sublink_index, descriptor);

                        self.range_count += 2;

                        /*
                        {
                            let mut link = self.link;
                            let mut index = 0;
                            let mut current_index = 0;

                            let mut end = 0;
                            while link != END_LINK && current_index < self.range_count {
                                let descriptor = read_descriptor(link, index);
                                let descriptor_end = descriptor
                                    .start
                                    .strict_add(descriptor.count.strict_mul(frame_size()));

                                crate::trace!("{descriptor:x?}");
                                assert!(end < descriptor_end);
                                end = descriptor_end;

                                self.next_location(&mut link, &mut index);
                                current_index += 1;
                            }
                        }
                        */

                        // The target descriptor was completely enclosed, which means that the
                        // target descriptor cannot and could not have overlapped with any other
                        // sublink descriptors because the list of descriptors is ordered and each
                        // descriptor controls a unique region.
                        return Ok(());
                    }
                    (true, false) => {
                        // The target descriptor consumes the upper part of the sublink descriptor.
                        assert!(lower_overlap_location.is_none());
                        lower_overlap_location = Some((current_link, sublink_index));
                    }
                    (false, true) => {
                        // The target descriptor consumes the lower part of the sublink descriptor.
                        assert!(upper_overlap_location.is_none());
                        upper_overlap_location = Some((current_link, sublink_index));
                    }
                    (false, false) => {
                        subsuming_overlap_location =
                            subsuming_overlap_location.or(Some((current_link, sublink_index)));
                        subsuming_overlap_count += 1;
                    }
                }

                sublink_index += 1;
                current_index += 1;
            }

            current_link = read_u64_at(current_link);
        }

        /*
        crate::trace!("Lower: {lower_overlap_location:x?}");
        crate::trace!("Inner: {subsuming_overlap_location:x?}");
        crate::trace!("Inner: {subsuming_overlap_count:x?}");
        crate::trace!("Upper: {upper_overlap_location:x?}");
        */

        if let Some((link, index)) = lower_overlap_location {
            let mut lower_descriptor = read_descriptor(link, index);

            lower_descriptor.count = (current_end - lower_descriptor.start) / frame_size();
            write_descriptor(link, index, lower_descriptor);
            // crate::trace!("handling lower-descriptor: {lower_descriptor:x?}");
        }

        if let Some((link, index)) = upper_overlap_location {
            let mut upper_descriptor = read_descriptor(link, index);

            let upper_descriptor_end = upper_descriptor
                .start
                .strict_add(upper_descriptor.count.strict_mul(frame_size()));

            upper_descriptor.start = current_end;
            upper_descriptor.count = (upper_descriptor_end - current_end) / frame_size();
            write_descriptor(link, index, upper_descriptor);
            // crate::trace!("handling upper-descriptor: {upper_descriptor:x?}");
        }

        if let Some((link, index)) = subsuming_overlap_location {
            let mut subsumed_descriptor = read_descriptor(link, index);

            subsumed_descriptor.start = current_start;
            subsumed_descriptor.count = (current_end - current_start) / frame_size();
            subsumed_descriptor.region_type = descriptor.region_type;
            write_descriptor(link, index, subsumed_descriptor);

            let mut write_link = link;
            let mut write_index = index;
            self.next_location(&mut write_link, &mut write_index);

            let mut read_link = link;
            let mut read_index = index;
            for _ in 0..subsuming_overlap_count {
                self.next_location(&mut read_link, &mut read_index);
            }

            for _ in 0..(subsuming_overlap_count - 1) {
                let descriptor = read_descriptor(read_link, read_index);
                write_descriptor(write_link, write_index, descriptor);

                self.next_location(&mut write_link, &mut write_index);
                self.next_location(&mut read_link, &mut read_index);
            }

            self.range_count -= subsuming_overlap_count - 1;
        } else {
            // Shift descriptors up 1.
            if self.range_count.strict_add(1) > self.max_capacity() {
                return Err(());
            }

            let (link, index) = if let Some(loc) = upper_overlap_location {
                loc
            } else {
                (current_link, current_index % self.descriptors_per_link)
            };
            self.shift_one_up(link, index);

            let descriptor = MemoryDescriptor {
                start: current_start,
                count: (current_end - current_start) / frame_size(),
                region_type: descriptor.region_type,
            };
            write_descriptor(link, index, descriptor);

            self.range_count += 1;
        }

        /*
        {
            let mut link = self.link;
            let mut index = 0;
            let mut current_index = 0;

            let mut end = 0;
            while link != END_LINK && current_index < self.range_count {
                let descriptor = read_descriptor(link, index);
                let descriptor_end = descriptor
                    .start
                    .strict_add(descriptor.count.strict_mul(frame_size()));

                crate::trace!("{descriptor:x?}");
                assert!(end < descriptor_end);
                end = descriptor_end;

                self.next_location(&mut link, &mut index);
                current_index += 1;
            }
        }
        */

        Ok(())
    }

    fn shift_one_up(&mut self, link: u64, index: u64) {
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
            carry_index = self.descriptors_per_link;
            self.next_location(&mut carry_link, &mut carry_index);
            if carry_link == END_LINK {
                break;
            }

            let storage = read_descriptor(carry_link, self.descriptors_per_link - 1);
            for _ in (0..(self.descriptors_per_link - 1)).rev() {
                let descriptor = read_descriptor(carry_link, carry_index);
                write_descriptor(carry_link, carry_index + 1, descriptor);
            }
            write_descriptor(carry_link, 0, carry);

            carry = storage;
        }
    }

    fn allocate_link(&mut self) {
        let mut link = self.link;
        let mut index = 0;
        let mut current_index = 0;
        while link != END_LINK && current_index <= self.range_count {
            let mut descriptor = read_descriptor(link, index);
            if descriptor.region_type != MemoryType::FREE
                || descriptor.count <= 1
                || descriptor.start == 0
            {
                self.next_location(&mut link, &mut index);
                current_index += 1;
                continue;
            }

            self.add_link(descriptor.start);

            descriptor.count = 1;
            descriptor.region_type = MemoryType::BOOTLOADER_RECLAIMABLE;
            self.try_insert_region(descriptor).unwrap();
        }
    }

    #[track_caller]
    fn add_link(&mut self, link: u64) {
        let mut previous_link = END_LINK;
        let mut current_link = self.link;
        while current_link != END_LINK {
            previous_link = current_link;
            current_link = read_u64_at(current_link);
        }

        if previous_link == END_LINK {
            self.link = link;
            write_u64_at(self.link, END_LINK);
        } else {
            write_u64_at(previous_link, link);
            write_u64_at(link, END_LINK);
        }

        crate::debug!("added link {link:#x}");
        self.link_count += 1;
    }

    fn find_link_init<I: Iterator<Item = MemoryDescriptor> + Clone + ExactSizeIterator>(
        &self,
        iter: I,
    ) -> Option<u64> {
        let frame_size = frame_size();
        for descriptor in iter.clone() {
            assert!(descriptor.start.is_multiple_of(frame_size));
            assert_ne!(descriptor.count, 0);
            if descriptor.region_type != MemoryType::FREE {
                continue;
            }

            'overlap_check: for frame in (0..descriptor.count)
                .map(|index| descriptor.start.strict_add(index.strict_mul(frame_size)))
            {
                if frame == 0 {
                    continue;
                }

                let start_0 = frame;
                let end_0 = start_0.strict_add(frame_size);

                for check_descriptor in iter.clone() {
                    if check_descriptor.region_type == MemoryType::FREE {
                        continue;
                    }

                    let start_1 = check_descriptor.start;
                    let end_1 = start_1.strict_add(check_descriptor.count.strict_mul(frame_size));
                    if start_0.max(start_1) < end_0.min(end_1) {
                        // The frame overlaps with this region and thus is not usable.
                        continue 'overlap_check;
                    }
                }

                for link_start in self.links() {
                    let start_1 = link_start;
                    let end_1 = start_1.strict_add(frame_size);
                    if start_0.max(start_1) < end_0.min(end_1) {
                        // The frame overlaps with this region and thus is not usable.
                        continue 'overlap_check;
                    }
                }

                return Some(frame);
            }
        }

        None
    }

    fn links(&self) -> LinkIter<'_> {
        LinkIter {
            _allocator: self,
            current: self.link,
        }
    }

    fn descriptors(&self) -> DescriptorIter<'_> {
        DescriptorIter {
            allocator: self,
            link: self.link,
            index: 0,
            total_index: 0,
        }
    }

    /// Given a `link` and a sublink `index`, this updates them to their next location.
    fn next_location(&self, link: &mut u64, index: &mut u64) {
        if *link == END_LINK {
            return;
        }

        if *index >= self.descriptors_per_link {
            *link = read_u64_at(*link);
            *index = 0;
            return;
        }

        *index += 1;
    }

    fn max_capacity(&self) -> u64 {
        self.link_count.strict_mul(self.descriptors_per_link)
    }
}

struct LinkIter<'allocator> {
    _allocator: &'allocator FrameAllocator,
    current: u64,
}

impl Iterator for LinkIter<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current & 0b1 == 0b1 {
            return None;
        }

        let value = self.current;
        self.current = read_u64_at(self.current);
        Some(value)
    }
}

struct DescriptorIter<'allocator> {
    allocator: &'allocator FrameAllocator,

    link: u64,
    index: u64,
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

fn read_descriptor(link: u64, index: u64) -> MemoryDescriptor {
    let array_offset = link.strict_add(usize_to_u64(mem::size_of::<u64>()));
    let entry_offset = index.strict_mul(DESCRIPTOR_SIZE);
    read_descriptor_at(array_offset.strict_add(entry_offset))
}

fn write_descriptor(link: u64, index: u64, descriptor: MemoryDescriptor) {
    let array_offset = link.strict_add(usize_to_u64(mem::size_of::<u64>()));
    let entry_offset = index.strict_mul(DESCRIPTOR_SIZE);
    write_descriptor_at(array_offset.strict_add(entry_offset), descriptor)
}

fn read_descriptor_at(physical_address: u64) -> MemoryDescriptor {
    let start_offset = usize_to_u64(offset_of!(MemoryDescriptor, start));
    let count_offset = usize_to_u64(offset_of!(MemoryDescriptor, count));
    let region_type_offset = usize_to_u64(offset_of!(MemoryDescriptor, region_type));

    let start = read_u64_at(physical_address.strict_add(start_offset));
    let count = read_u64_at(physical_address.strict_add(count_offset));
    let region_type = MemoryType(read_u32_at(physical_address.strict_add(region_type_offset)));

    MemoryDescriptor {
        start,
        count,
        region_type,
    }
}

fn write_descriptor_at(physical_address: u64, descriptor: MemoryDescriptor) {
    let start_offset = usize_to_u64(offset_of!(MemoryDescriptor, start));
    let count_offset = usize_to_u64(offset_of!(MemoryDescriptor, count));
    let region_type_offset = usize_to_u64(offset_of!(MemoryDescriptor, region_type));

    write_u64_at(physical_address.strict_add(start_offset), descriptor.start);
    write_u64_at(physical_address.strict_add(count_offset), descriptor.count);
    write_u32_at(
        physical_address.strict_add(region_type_offset),
        descriptor.region_type.0,
    );
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Fnv1aHash(u64);

impl Hasher for Fnv1aHash {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = self.0 ^ u64::from(byte);
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
