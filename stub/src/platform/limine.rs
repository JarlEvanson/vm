//! Support for booting from the Limine boot protocol.

use limine::{BaseRevisionTag, RequestsEndMarker, RequestsStartMarker};
use sync::ControlledModificationCell;

/// Indicates the start of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.start")]
static REQUESTS_START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

/// Tag used to communicate the information regarding the base revision of the Limine protocol.
#[used]
#[unsafe(link_section = ".limine.base_tag")]
static BASE_REVISION_TAG: ControlledModificationCell<BaseRevisionTag> =
    ControlledModificationCell::new(BaseRevisionTag::new_current());

/// Indicates the end of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.end")]
static REQUESTS_END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

/// Entry point for Rust when booted using the Limine boot protocol.
pub extern "C" fn limine_main() -> ! {
    let base_revision_tag = BASE_REVISION_TAG.get();
    if !base_revision_tag.is_supported() {
        // If the base revision this executable was loaded using is greater than or equal to 3,
        // then [`BaseRevisionTag::loaded_revision`] contains the base revision used to load the
        // executable. Otherwise, the base revision must be either 0, 1, or 2.
        if let Some(loaded_revision) = base_revision_tag.loaded_revision() {
            panic!("Loaded using unsupported base revision {loaded_revision}",)
        }

        panic!("Loaded using unsupported base revision (possible revisions are 0, 1, and 2)")
    }

    crate::debug!("Image Start: {:#x}", crate::util::image_start());

    loop {
        core::hint::spin_loop()
    }
}
