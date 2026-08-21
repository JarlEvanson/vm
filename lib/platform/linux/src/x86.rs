//! Definitions for the `i686` and `x86_64` linux boot protocol.

use core::mem;

/// The header used for configuration and setup of a valid `linux` boot protocol image.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Header {
    /// 0x1F1: The size of the setup in 512-byte sectors (0 means 4)
    pub setup_sects: u8,
    /// 0x1F2: If set, the root filesystem is mounted readonly (Deprecated)
    pub root_flags: u16,
    /// 0x1F4: The size of the 32-bit protected-mode code in 16-byte paragraphs
    pub syssize: u32,
    /// 0x1F8: DO NOT USE - for bootsect.S use only
    pub ram_size: u16,
    /// 0x1FA: Video mode control
    pub vid_mode: u16,
    /// 0x1FC: Default root device number (Deprecated)
    pub root_dev: u16,
    /// 0x1FE: 0xAA55 magic number
    pub boot_flag: u16,
    /// 0x200: Jump instruction (0xEB followed by signed offset relative to 0x202)
    pub jump: u16,
    /// 0x202: Magic signature "HdrS" (0x53726448)
    pub header: u32,
    /// 0x206: Boot protocol version supported (e.g., 0x0204 for 2.04)
    pub version: u16,
    /// 0x208: Boot loader hook pointer
    pub realmode_swtch: u32,
    /// 0x20C: The load-low segment (0x1000) (Obsolete)
    pub start_sys_seg: u16,
    /// 0x20E: Pointer to kernel version string (relative to 0x200 offset)
    pub kernel_version: u16,
    /// 0x210: Boot loader identifier
    pub type_of_loader: u8,
    /// 0x211: Boot protocol option flags (bitmask)
    pub loadflags: u8,
    /// 0x212: Move to high memory size (used with hooks for protocol 2.00-2.01)
    pub setup_move_size: u16,
    /// 0x214: Protected mode entry point address / boot loader hook
    pub code32_start: u32,
    /// 0x218: initrd load address (32-bit linear address set by boot loader)
    pub ramdisk_image: u32,
    /// 0x21C: initrd size (set by boot loader)
    pub ramdisk_size: u32,
    /// 0x220: DO NOT USE - for bootsect.S use only
    pub bootsect_kludge: u32,
    /// 0x224: Free memory after setup end (offset from beginning of real-mode code minus 0x0200)
    pub heap_end_ptr: u16,
    /// 0x226: Extended boot loader version
    pub ext_loader_ver: u8,
    /// 0x227: Extended boot loader ID
    pub ext_loader_type: u8,
    /// 0x228: 32-bit pointer to the kernel command line
    pub cmd_line_ptr: u32,
    /// 0x22C: Highest legal initrd address
    pub initrd_addr_max: u32,
    /// 0x230: Physical address alignment required for kernel (if relocatable)
    pub kernel_alignment: u32,
    /// 0x234: Non-zero if the protected-mode kernel is relocatable
    pub relocatable_kernel: u8,
    /// 0x235: Minimum alignment required as a power of two
    pub min_alignment: u8,
    /// 0x236: Boot protocol option flags (bitmask)
    pub xloadflags: u16,
    /// 0x238: Maximum size of the kernel command line without the terminating zero
    pub cmdline_size: u32,
    /// 0x24C: Hardware subarchitecture environment type
    pub hardware_subarch: u32,
    /// 0x240: Subarchitecture-specific data pointer
    pub hardware_subarch_data: u64,
    /// 0x248: Offset of kernel payload from protected-mode code start
    pub payload_offset: u32,
    /// 0x24C: Length of the kernel payload
    pub payload_length: u32,
    /// 0x250: 64-bit physical pointer to linked list of `SetupData`
    pub setup_data: u64,
    /// 0x258: Preferred loading address for the kernel
    pub pref_address: u64,
    /// 0x260: Linear memory required during initialization
    pub init_size: u32,
    /// 0x264: Offset of the EFI handover entry point
    pub handover_offset: u32,
    /// 0x268: Offset of the kernel_info structure
    pub kernel_info_offset: u32,
}

impl Header {
    /// The file offset at which this [`Header`] should appear.
    pub const BASE_OFFSET: usize = 0x1F1;
}

// The Linux x86 "zero page" structure (`boot_params`).
///
/// This structure acts as the primary configuration exchange between the
/// bootloader and the Linux kernel.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BootParams {
    /// 0x000: Hardware screen information (video mode, columns, lines, etc.)
    pub screen_info: ScreenInfo,
    /// 0x040: APM BIOS information
    pub apm_bios_info: [u8; 20],
    /// 0x054: Padding
    pub _padding_2: [u8; 4],
    /// 0x058: Address of tboot shared page.
    pub tboot_addr: u64,
    /// 0x060: Intel SpeedStep (IST) information
    pub ist_info: [u8; 16],
    /// 0x070: ACPI RSDP address
    pub acpi_rsdp_addr: u64,
    /// 0x078: Padding
    pub _padding_3: [u8; 8],
    /// 0x080: HDIO_GETGEO biological disk parameters
    pub hd0_info: [u8; 16],
    /// 0x090: HDIO_GETGEO biological disk parameters
    pub hd1_info: [u8; 16],
    /// 0x0A0: System descriptor table
    pub sys_desc_table: [u8; 16],
    /// 0x0B0: OFW header
    pub olpc_ofw_header: [u8; 16],
    /// 0x0C0: Extension ramdisk image address
    pub ext_ramdisk_image: u32,
    /// 0x0C4: Extension ramdisk image size
    pub ext_ramdisk_size: u32,
    /// 0x0C8: Extension command line pointer
    pub ext_cmd_line_ptr: u32,
    /// 0x0CC: Padding
    pub _padding_4: [u8; 112],
    /// 0x13C: Address of the CC blob
    pub cc_blob_address: u32,
    /// 0x140: EDID for the current screen
    pub edid_info: [u8; 128],
    /// 0x1C0: Information about the EFI implementation.
    pub efi_info: EfiInfo,
    /// 0x1E0:
    pub alt_mem_k: u32,
    /// 0x1E4:
    pub scratch: u32,
    /// 0x1E8:
    pub e820_entries: u8,
    /// 0x1E9:
    pub eddbuf_entries: u8,
    /// 0x1EA:
    pub edd_mbr_sig_buf_entries: u8,
    /// 0x1EB:
    pub kbd_status: u8,
    /// 0x1EC:
    pub secure_boot: u8,
    /// 0x1ED: Padding
    pub _padding_5: [u8; 2],
    /// 0x1EF
    pub sentinel: u8,
    /// 0x1F0: Padding
    pub _padding_6: [u8; 1],
    /// 0x1F1: The embedded [`Header`].
    pub hdr: Header,
    /// 0x???: Padding
    pub _padding_7: [u8; const { 0x290 - 0x1F1 - mem::size_of::<Header>() }],
    /// 0x290:
    pub edd_mbr_sig_buffer: [u32; 16],
    /// 0x2D0:
    pub e820_table: [E820Entry; 128],
    /// 0xCD0: Padding
    pub _padding_8: [u8; 48],
    /// 0xD00:
    pub edd_info: [u8; 492],
    /// 0xEEC: Padding
    pub _padding_9: [u8; 276],
}

/// Video information.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenInfo {
    /// Cursor column position at boot time (0-indexed).
    pub original_x: u8,
    /// Cursor row position at boot time (0-indexed).
    pub original_y: u8,

    /// Extended memory size in Kilobytes.
    pub ext_mem_k: u16,

    /// Active video page number.
    pub orig_video_page: u16,
    /// Video mode selected at boot time (e.g., standard BIOS modes or VESA modes).
    pub orig_video_mode: u8,
    /// Number of text columns on the screen.
    pub orig_video_cols: u8,
    /// Legacy video configuration flags.
    pub flags: u8,
    /// Padding byte.
    pub _unused_2: u8,
    /// The EGA `BX` register value, often indicating video memory size or configuration.
    pub orig_video_ega_bx: u16,
    /// Padding bytes.
    pub _unused_3: u16,
    /// Number of text rows/lines on the screen.
    pub orig_video_lines: u8,
    /// Legacy video configuration flags.
    pub orig_video_is_vga: VideoType,
    /// Font pixel height (points per character).
    pub orig_video_points: u16,

    /// Linear Framebuffer (LFB) width in pixels.
    pub lfb_width: u16,
    /// Linear Framebuffer (LFB) height in pixels.
    pub lfb_height: u16,
    /// Linear Framebuffer (LFB) color depth (bits per pixel, e.g., 16, 24, 32).
    pub lfb_depth: u16,
    /// 32-bit physical base address of the Linear Framebuffer.
    pub lfb_base: u32,
    /// Size of the Linear Framebuffer in bytes.
    pub lfb_size: u32,
    /// Command line magic number used by the bootloader.
    pub cl_magic: u16,
    /// Command line offset relative to the setup segment.
    pub cl_offset: u16,
    /// Number of bytes per scanline (pitch/stride).
    pub lfb_line_length: u16,
    /// Size of the red color component mask in bits.
    pub red_size: u8,
    /// Bit position of the red color component within the pixel.
    pub red_pos: u8,
    /// Size of the green color component mask in bits.
    pub green_size: u8,
    /// Bit position of the green color component within the pixel.
    pub green_pos: u8,
    /// Size of the blue color component mask in bits.
    pub blue_size: u8,
    /// Bit position of the blue color component within the pixel.
    pub blue_pos: u8,
    /// Size of the reserved/alpha component mask in bits.
    pub reserved_size: u8,
    /// Bit position of the reserved/alpha component within the pixel.
    pub reserved_pos: u8,

    /// VESA Protected Mode interface segment descriptor.
    pub vesapm_seg: u16,
    /// VESA Protected Mode interface offset.
    pub vesapm_off: u16,
    /// Number of memory pages/buffers available.
    pub pages: u16,
    /// VESA capabilities flags.
    pub vesa_attributes: u16,
    /// Raw screen capabilities.
    pub capabilities: VideoCapabilities,
    /// High 32 bits of the 64-bit physical base address of the Linear Framebuffer (if applicable).
    pub ext_lfb_base: u32,
    /// Reserved for future extensions.
    pub _reserved: [u8; 2],
}

const _: () = assert!(core::mem::size_of::<ScreenInfo>() == 64);

/// The type of screen that the associated [`ScreenInfo`] describes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VideoType(pub u8);

/// Various capabilities and attributes associated with the screen.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VideoCapabilities(pub u32);

impl VideoCapabilities {
    /// Quirks should be skipped.
    pub const SKIP_QUIRKS: Self = Self(1 << 0);
    /// The framebuffer base is 64-bits.
    pub const BASE_64_BIT: Self = Self(1 << 1);
}

/// Information regarding the EFI implementation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EfiInfo {
    /// Signature of the loader.
    pub loader_signature: u32,
    /// Low bits of the system table pointer.
    pub system_table: u32,
    /// Size, in bytes, of each memory map descriptor.
    pub memory_descriptor_size: u32,
    /// Format version of the memory map descriptors.
    pub memory_descriptor_version: u32,
    /// Low bits of the memory map pointer.
    pub memory_map: u32,
    /// Total size, in bytes, of the memory map.
    pub memmap_size: u32,
    /// High bits of the system table pointer.
    pub system_table_high: u32,
    /// High bits of the memory map pointer.
    pub memory_map_high: u32,
}

/// A single entry in the E820 memory map.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct E820Entry {
    /// Physical address of the start of the memory range
    pub addr: u64,
    /// Length of the memory range in bytes
    pub size: u64,
    /// Type of memory range (1 = Available RAM, 2 = Reserved, etc.)
    pub entry_type: u32,
}

/// Extensible [`SetupData`] list node.
#[repr(C)]
pub struct SetupData {
    /// Physical address of the next node (or zero if there are no more nodes).
    pub next: u64,
    /// [`SetupData`] datatype identifier.
    pub setup_type: SetupType,
    /// Length of the following [`SetupData`] payload.
    pub len: u32,
    /// Location of data for this node.
    pub _data: [u8; 0],
}

/// Extensible [`SetupData`] indirect data node.
#[repr(C)]
pub struct SetupDataIndirect {
    /// [`SetupData`] datatype identifier.
    pub setup_type: SetupType,
    /// Reserved.
    pub _reserved: u32,
    /// Length of the indirect data section.
    pub length: u64,
    /// Physical address of the indirect data section.
    pub address: u64,
}

/// The type of data a [`SetupData`] node demarcates.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct SetupType(pub u32);

impl SetupType {
    /// No setup data specified.
    pub const NONE: Self = Self(0);

    /// An additional [`E820Entry`].
    pub const E820_EXT: Self = Self(1);

    /// The system Device Tree Blob (DTB).
    pub const DTB: Self = Self(2);

    /// PCI-related setup information.
    pub const PCI: Self = Self(3);

    /// EFI-specific setup data.
    pub const EFI: Self = Self(4);

    /// Device properties for Apple hardware.
    pub const APPLE_PROPERTIES: Self = Self(5);

    /// Jailhouse hypervisor configuration data.
    pub const JAILHOUSE: Self = Self(6);

    /// Confidential Computing (CC) blob (e.g., SEV-SNP/TDX measurements).
    pub const CC_BLOB: Self = Self(7);

    /// Integrity Measurement Architecture (IMA) log data.
    pub const IMA: Self = Self(8);

    /// Random Number Generator (RNG) seed for early boot entropy.
    pub const RNG_SEED: Self = Self(9);

    /// Kexec Handover (KHO) data for hot-reboot state preservation.
    pub const KEXEC_KHO: Self = Self(10);

    /// The maximum valid enum base value.
    pub const ENUM_MAX: Self = Self::KEXEC_KHO;

    /// Flag indicating that the data is stored indirectly (the payload contains a pointer/length).
    pub const INDIRECT: u32 = 1 << 31;

    /// The absolute maximum value possible, including the indirect flag.
    pub const TYPE_MAX: Self = Self(Self::ENUM_MAX.0 | Self::INDIRECT);

    /// Returns the raw underlying integer value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Returns the base enum value with the `INDIRECT` flag stripped out.
    #[inline]
    pub const fn base_type(self) -> u32 {
        self.0 & !Self::INDIRECT
    }

    /// Returns `true` if the provided `base_type` matches the base enum vlaue of `self`.
    #[inline]
    pub const fn is_base_type(self, base_type: Self) -> bool {
        self.base_type() == base_type.base_type()
    }

    /// Checks if the `INDIRECT` flag is set on this setup type.
    #[inline]
    pub const fn is_indirect(self) -> bool {
        (self.0 & Self::INDIRECT) != 0
    }
}
