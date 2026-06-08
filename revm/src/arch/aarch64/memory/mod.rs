//! Architectural memory manipulation and introspection functionality for `aarch64`.

use crate::arch::capabilities::arch_capability_support;

/// Computes the page frame size to be used for this application
pub fn compute_page_frame_size() -> usize {
    let capabilities = arch_capability_support();

    // Gather all supported granules, extracting the width for easy sorting.
    let mut granules = [
        capabilities.granule_4().map(|width| (width, 4096)),
        capabilities.granule_16().map(|width| (width, 16384)),
        capabilities.granule_64().map(|width| (width, 65536)),
    ];

    granules.sort_unstable_by(|a, b| {
        match (a, b) {
            (Some(a_val), Some(b_val)) => {
                // 1. Sort by highest physical address width descending (b vs a)
                // 2. If widths are equal, tie-break by smallest page size ascending (a vs b)
                b_val.0.cmp(&a_val.0).then_with(|| a_val.1.cmp(&b_val.1))
            }
            // Push valid configurations (Some) to the front, unsupported (None) to the back
            (Some(_), None) => core::cmp::Ordering::Less,
            (None, Some(_)) => core::cmp::Ordering::Greater,
            (None, None) => core::cmp::Ordering::Equal,
        }
    });

    granules[0]
        .expect("address translation must be supported")
        .1
}
