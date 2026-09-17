use core::{
    alloc::Layout,
    mem,
    ptr::NonNull,
    sync::atomic::{AtomicU8, AtomicUsize},
};
use std::alloc::{alloc, dealloc, handle_alloc_error};

use crate::{Descriptor, LogBuffer};

pub struct AllocatedLogBuffer(LogBuffer);

impl AllocatedLogBuffer {
    pub fn new(data_capacity: usize, descriptor_capacity: usize, starter_message: &str) -> Self {
        assert_ne!(data_capacity, 0);
        assert_ne!(descriptor_capacity, 0);

        let data_layout = Layout::array::<AtomicU8>(data_capacity)
            .and_then(|layout| layout.align_to(mem::align_of::<AtomicUsize>()))
            .expect("requested LogBuffer data size is too large");
        let descriptor_layout = Layout::array::<Descriptor>(descriptor_capacity)
            .expect("requested LogBuffer descriptor count is too large");

        let data_buffer = unsafe { alloc(data_layout) };
        let descriptor_buffer = unsafe { alloc(descriptor_layout) };

        let buffers = [
            (data_buffer, data_layout),
            (descriptor_buffer, descriptor_layout),
        ];
        if let Some(&(_, layout)) = buffers.iter().find(|(buffer, _)| buffer.is_null()) {
            for &(buffer, layout) in buffers.iter().filter(|(buffer, _)| !buffer.is_null()) {
                if layout.size() == 0 {
                    continue;
                }
                unsafe { dealloc(buffer, layout) }
            }

            handle_alloc_error(layout);
        }

        let logbuffer = unsafe {
            LogBuffer::new(
                NonNull::new(data_buffer.cast::<AtomicU8>()).unwrap(),
                data_capacity,
                NonNull::new(descriptor_buffer.cast::<Descriptor>()).unwrap(),
                descriptor_capacity,
                starter_message,
            )
        };

        Self(logbuffer)
    }
}

impl core::ops::Deref for AllocatedLogBuffer {
    type Target = LogBuffer;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::ops::DerefMut for AllocatedLogBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for AllocatedLogBuffer {
    fn drop(&mut self) {
        let data_layout = Layout::array::<AtomicU8>(self.0.data_capacity)
            .and_then(|layout| layout.align_to(mem::align_of::<AtomicUsize>()))
            .unwrap();
        let descriptor_layout = Layout::array::<Descriptor>(self.descriptor_capacity).unwrap();

        unsafe { dealloc(self.0.data_ptr.as_ptr().cast::<u8>(), data_layout) };
        unsafe {
            dealloc(
                self.0.descriptor_ptr.as_ptr().cast::<u8>(),
                descriptor_layout,
            )
        };
    }
}
