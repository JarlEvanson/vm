//! Definitions of various graphics APIs and a graphical console based on those APIs.

pub mod font {
    //! Defines the graphical font interfaces for `tvm_loader`.

    use core::mem;
    use font_map::{FontMap, FontMapEntry};
    use glyph::GlyphArray;

    /// The default font's [`GlyphArray`].
    pub const GLYPH_ARRAY: GlyphArray = {
        const GLYPHS: &[u8] = include_bytes!("../../../../target/glyph_array.bin");

        match GlyphArray::from_dump(GLYPHS) {
            Some(glyph_array) => glyph_array,
            None => panic!("TVM_LOADER_GLYPH_ARRAY does not point to a valid dump"),
        }
    };

    /// The default font's [`FontMap`].
    pub const FONT_MAP: FontMap = {
        const FONT_MAP_BYTES: &Aligned<FontMapEntry, [u8]> = &Aligned {
            aligner: [],
            value: *include_bytes!("../../../../target/font_map.bin"),
        };

        /// Structure that only changes the alignment of the wrapped value.
        #[repr(C)]
        struct Aligned<A, T: ?Sized> {
            /// Zero-sized field used to align `value`.
            aligner: [A; 0],
            /// The value wrapped by the field.
            value: T,
        }

        // SAFETY:
        //
        // [`FONT_MAP_BYTES`] is required to contain bytes and is aligned to [`FontMapEntry`]'s
        // requirements.
        let font_map_entries = unsafe {
            core::slice::from_raw_parts(
                FONT_MAP_BYTES.value.as_ptr().cast::<FontMapEntry>(),
                FONT_MAP_BYTES.value.len() / mem::size_of::<FontMapEntry>(),
            )
        };

        FontMap::new(font_map_entries)
    };

    pub use font::*;
}
