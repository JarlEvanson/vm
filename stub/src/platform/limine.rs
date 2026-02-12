//! Support for booting from the Limine boot protocol.

use limine::{BASE_REVISION, BASE_REVISION_MAGIC_0, BASE_REVISION_MAGIC_1, BaseRevisionTag};
use sync::ControlledModificationCell;

/// Indicates the start of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.start")]
static REQUESTS_START_MARKER: [u64; 4] = limine::REQUESTS_START_MARKER;

/// Tag used to communicate the information regarding the base revision of the Limine protocol.
#[used]
#[unsafe(link_section = ".limine.base_tag")]
static BASE_REVISION_TAG: ControlledModificationCell<BaseRevisionTag> =
    ControlledModificationCell::new(BaseRevisionTag {
        magic: BASE_REVISION_MAGIC_0,
        loaded_revision: BASE_REVISION_MAGIC_1,
        supported_revision: BASE_REVISION,
    });

/// Indicates the end of the Limine boot protocol request zone.
#[used]
#[unsafe(link_section = ".limine.end")]
static REQUESTS_END_MARKER: [u64; 2] = limine::REQUESTS_END_MARKER;

/// Entry point for Rust when booted using the Limine boot protocol.
pub extern "C" fn limine_main() -> ! {
    if BASE_REVISION_TAG.get().supported_revision == BASE_REVISION {
        // If the base revision this executable was loaded using is greater than or equal to 3,
        // then [`BaseRevisionTag::loaded_revision`] contains the base revision used to load the
        // executable. Otherwise, the base revision must be either 0, 1, or 2.
        if BASE_REVISION_TAG.get().loaded_revision != BASE_REVISION_MAGIC_1 {
            panic!(
                "Loaded using unsupported base revision {}",
                BASE_REVISION_TAG.get().loaded_revision
            )
        } else {
            panic!("Loaded using unsupported base revision (possible revisions are 0, 1, and 2)")
        }
    }

    crate::debug!("{:#x}", crate::util::image_start());
    match crate::stub_main() {
        Ok(()) => {}
        Err(error) => crate::error!("error loading from Limine: {error}"),
    };

    loop {
        core::hint::spin_loop()
    }
}
