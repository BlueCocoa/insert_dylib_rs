pub const MH_CIGAM_64: u32 = 0xcffaedfe;
pub const MH_MAGIC_64: u32 = 0xfeedfacf;
pub const MH_CIGAM: u32 = 0xcefaedfe;
pub const MH_MAGIC: u32 = 0xfeedface;
pub const FAT_MAGIC: u32 = 0xcafebabe;
pub const FAT_CIGAM: u32 = 0xbebafeca;

pub const LC_REQ_DYLD: u32 = 0x80000000;
pub const LC_SEGMENT: u32 = 0x01;
pub const LC_SYMTAB: u32 = 0x02;
pub const LC_SEGMENT_64: u32 = 0x19;
pub const LC_CODE_SIGNATURE: u32 = 0x1d;
pub const LC_LOAD_DYLIB: u32 = 0x0c;
pub const LC_LOAD_WEAK_DYLIB: u32 = 0x18 | LC_REQ_DYLD;
