use super::prelude::*;

macro_rules! swap_bytes {
    ($self:ident, $field_name:ident) => {
        $self.$field_name = $self.$field_name.swap_bytes();
    };
}

pub trait FixLoadCommandEndian {
    fn fix_endian(&mut self);
}

#[derive(Debug)]
pub struct FatHeader {
    pub magic: u32,
    pub nfat_arch: u32,
}

impl FatHeader {
    pub fn from(buffer: [u8; 8], is_little_endian: bool) -> FatHeader {
        let header_buffer: [u32; 2] =
            unsafe { std::mem::transmute_copy::<[u8; 8], [u32; 2]>(&buffer) };
        let mut fat_header = FatHeader {
            magic: header_buffer[0],
            nfat_arch: header_buffer[1],
        };

        if is_little_endian {
            fat_header.fix_endian()
        }

        fat_header
    }

    pub fn to_u8(&self) -> [u8; 8] {
        let mut data: [u32; 2] = [0u32; 2];
        data[0] = self.magic;
        data[1] = self.nfat_arch;

        unsafe { std::mem::transmute_copy::<[u32; 2], [u8; 8]>(&data) }
    }
}

impl FixLoadCommandEndian for FatHeader {
    fn fix_endian(&mut self) {
        swap_bytes!(self, magic);
        swap_bytes!(self, nfat_arch);
    }
}

#[derive(Debug)]
pub struct FatArch {
    pub cputype: u32,
    pub cpusubtype: u32,
    pub offset: u32,
    pub size: u32,
    pub align: u32,
}

impl FatArch {
    pub fn from(buffer: [u8; 20], is_little_endian: bool) -> FatArch {
        let arch_buffer: [u32; 5] =
            unsafe { std::mem::transmute_copy::<[u8; 20], [u32; 5]>(&buffer) };
        let mut fat_arch = FatArch {
            cputype: arch_buffer[0],
            cpusubtype: arch_buffer[1],
            offset: arch_buffer[2],
            size: arch_buffer[3],
            align: arch_buffer[4],
        };

        if is_little_endian {
            fat_arch.fix_endian()
        }

        fat_arch
    }

    pub fn to_u8(&self) -> [u8; 20] {
        let mut data: [u32; 5] = [0u32; 5];
        data[0] = self.cputype;
        data[1] = self.cpusubtype;
        data[2] = self.offset;
        data[3] = self.size;
        data[4] = self.align;

        unsafe { std::mem::transmute_copy::<[u32; 5], [u8; 20]>(&data) }
    }
}

impl FixLoadCommandEndian for FatArch {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cputype);
        swap_bytes!(self, cpusubtype);
        swap_bytes!(self, offset);
        swap_bytes!(self, size);
        swap_bytes!(self, align);
    }
}

#[derive(Debug)]
pub struct MachHeader {
    pub magic: u32,
    pub cputype: u32,
    pub cpusubtype: u32,
    pub filetype: u32,
    pub ncmds: u32,
    pub sizeofcmds: u32,
    pub flags: u32,
    pub reserved: u32,
}

impl MachHeader {
    pub fn from(buffer: [u8; 32]) -> MachHeader {
        let header_buffer: [u32; 8] =
            unsafe { std::mem::transmute_copy::<[u8; 32], [u32; 8]>(&buffer) };
        let magic = header_buffer[0].swap_bytes();
        let mut mach_header = MachHeader {
            magic: header_buffer[0],
            cputype: header_buffer[1],
            cpusubtype: header_buffer[2],
            filetype: header_buffer[3],
            ncmds: header_buffer[4],
            sizeofcmds: header_buffer[5],
            flags: header_buffer[6],
            reserved: header_buffer[7],
        };

        if !MachHeader::is_little_endian(magic) {
            mach_header.fix_endian();
        }

        mach_header
    }

    pub fn is_little_endian(magic: u32) -> bool {
        match magic {
            FAT_CIGAM | MH_CIGAM_64 | MH_CIGAM => true,
            FAT_MAGIC | MH_MAGIC_64 | MH_MAGIC => false,
            _ => panic!("Unknown MachO magic"),
        }
    }

    pub fn to_u8(&self) -> [u8; 32] {
        let mut data: [u32; 8] = [0u32; 8];
        data[0] = self.magic;
        data[1] = self.cputype;
        data[2] = self.cpusubtype;
        data[3] = self.filetype;
        data[4] = self.ncmds;
        data[5] = self.sizeofcmds;
        data[6] = self.flags;
        data[7] = self.reserved;

        unsafe { std::mem::transmute_copy::<[u32; 8], [u8; 32]>(&data) }
    }

    pub fn len(&self) -> u64 {
        match self.magic {
            MH_MAGIC | MH_CIGAM => 28,
            MH_MAGIC_64 | MH_CIGAM_64 => 32,
            _ => panic!("Unknown MachO magic"),
        }
    }
}

impl FixLoadCommandEndian for MachHeader {
    fn fix_endian(&mut self) {
        swap_bytes!(self, magic);
        swap_bytes!(self, cputype);
        swap_bytes!(self, cpusubtype);
        swap_bytes!(self, filetype);
        swap_bytes!(self, ncmds);
        swap_bytes!(self, sizeofcmds);
        swap_bytes!(self, flags);
        swap_bytes!(self, reserved);
    }
}

#[derive(Debug)]
pub struct SegmentCommand {
    pub cmd: u32,
    pub cmdsize: u32,
    pub segname: [u8; 16],
    pub vmaddr: u32,
    pub vmsize: u32,
    pub fileoff: u32,
    pub filesize: u32,
    pub maxprot: u32,
    pub initprot: u32,
    pub nsects: u32,
    pub flags: u32,
}

impl SegmentCommand {
    pub fn default() -> SegmentCommand {
        SegmentCommand {
            cmd: 0,
            cmdsize: 0,
            segname: [0u8; 16],
            vmaddr: 0,
            vmsize: 0,
            fileoff: 0,
            filesize: 0,
            maxprot: 0,
            initprot: 0,
            nsects: 0,
            flags: 0,
        }
    }

    pub fn from(buffer: [u8; 56], is_little_endian: bool) -> SegmentCommand {
        let sc_buffer: [u32; 14] =
            unsafe { std::mem::transmute_copy::<[u8; 56], [u32; 14]>(&buffer) };
        let mut segment_command = SegmentCommand::default();
        segment_command.cmd = sc_buffer[0];
        segment_command.cmdsize = sc_buffer[1];
        for i in 0..16usize {
            segment_command.segname[i] = buffer[8 + i];
        }
        segment_command.vmaddr = sc_buffer[6];
        segment_command.vmsize = sc_buffer[7];
        segment_command.fileoff = sc_buffer[8];
        segment_command.filesize = sc_buffer[9];
        segment_command.maxprot = sc_buffer[10];
        segment_command.initprot = sc_buffer[11];
        segment_command.nsects = sc_buffer[12];
        segment_command.flags = sc_buffer[13];

        if is_little_endian {
            segment_command.fix_endian();
        }

        segment_command
    }

    pub fn to_u8(&self) -> [u8; 56] {
        let mut data: [u32; 14] = [0u32; 14];
        let segname_data: [u32; 4] =
            unsafe { std::mem::transmute_copy::<[u8; 16], [u32; 4]>(&self.segname) };
        data[0] = self.cmd;
        data[1] = self.cmdsize;
        data[2] = segname_data[0];
        data[3] = segname_data[1];
        data[4] = segname_data[2];
        data[5] = segname_data[3];
        data[6] = self.vmaddr;
        data[7] = self.vmsize;
        data[8] = self.fileoff;
        data[9] = self.filesize;
        data[10] = self.maxprot;
        data[11] = self.initprot;
        data[12] = self.nsects;
        data[13] = self.flags;

        unsafe { std::mem::transmute_copy::<[u32; 14], [u8; 56]>(&data) }
    }
}

impl FixLoadCommandEndian for SegmentCommand {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cmd);
        swap_bytes!(self, cmdsize);
        swap_bytes!(self, vmaddr);
        swap_bytes!(self, vmsize);
        swap_bytes!(self, fileoff);
        swap_bytes!(self, filesize);
        swap_bytes!(self, maxprot);
        swap_bytes!(self, initprot);
        swap_bytes!(self, nsects);
        swap_bytes!(self, flags);
    }
}

#[derive(Debug)]
pub struct SegmentCommand64 {
    pub cmd: u32,
    pub cmdsize: u32,
    pub segname: [u8; 16],
    pub vmaddr: u64,
    pub vmsize: u64,
    pub fileoff: u64,
    pub filesize: u64,
    pub maxprot: u32,
    pub initprot: u32,
    pub nsects: u32,
    pub flags: u32,
}

impl SegmentCommand64 {
    pub fn default() -> SegmentCommand64 {
        SegmentCommand64 {
            cmd: 0,
            cmdsize: 0,
            segname: [0u8; 16],
            vmaddr: 0,
            vmsize: 0,
            fileoff: 0,
            filesize: 0,
            maxprot: 0,
            initprot: 0,
            nsects: 0,
            flags: 0,
        }
    }

    pub fn from(buffer: [u8; 72], is_little_endian: bool) -> SegmentCommand64 {
        let sc_buffer: [u32; 18] =
            unsafe { std::mem::transmute_copy::<[u8; 72], [u32; 18]>(&buffer) };
        let mut segment_command = SegmentCommand64::default();
        segment_command.cmd = sc_buffer[0];
        segment_command.cmdsize = sc_buffer[1];
        for i in 0..16usize {
            segment_command.segname[i] = buffer[8 + i];
        }
        segment_command.vmaddr = ((sc_buffer[7] as u64) << 32) + (sc_buffer[6] as u64);
        segment_command.vmsize = ((sc_buffer[9] as u64) << 32) + (sc_buffer[8] as u64);
        segment_command.fileoff = ((sc_buffer[11] as u64) << 32) + (sc_buffer[10] as u64);
        segment_command.filesize = ((sc_buffer[13] as u64) << 32) + (sc_buffer[12] as u64);
        segment_command.maxprot = sc_buffer[14];
        segment_command.initprot = sc_buffer[15];
        segment_command.nsects = sc_buffer[16];
        segment_command.flags = sc_buffer[17];

        if is_little_endian {
            segment_command.fix_endian();
        }

        segment_command
    }

    pub fn to_u8(&self) -> [u8; 72] {
        let mut data: [u32; 18] = [0u32; 18];
        let segname_data: [u32; 4] =
            unsafe { std::mem::transmute_copy::<[u8; 16], [u32; 4]>(&self.segname) };
        data[0] = self.cmd;
        data[1] = self.cmdsize;
        data[2] = segname_data[0];
        data[3] = segname_data[1];
        data[4] = segname_data[2];
        data[5] = segname_data[3];

        let vmaddr_data: [u32; 2] = unsafe { std::mem::transmute_copy(&self.vmaddr) };
        data[6] = vmaddr_data[0];
        data[7] = vmaddr_data[1];
        let vmsize_data: [u32; 2] = unsafe { std::mem::transmute_copy(&self.vmsize) };
        data[8] = vmsize_data[0];
        data[9] = vmsize_data[1];
        let fileoff_data: [u32; 2] = unsafe { std::mem::transmute_copy(&self.fileoff) };
        data[10] = fileoff_data[0];
        data[11] = fileoff_data[1];
        let filesize_data: [u32; 2] = unsafe { std::mem::transmute_copy(&self.filesize) };
        data[12] = filesize_data[0];
        data[13] = filesize_data[1];

        data[14] = self.maxprot;
        data[15] = self.initprot;
        data[16] = self.nsects;
        data[17] = self.flags;

        unsafe { std::mem::transmute_copy::<[u32; 18], [u8; 72]>(&data) }
    }
}

impl FixLoadCommandEndian for SegmentCommand64 {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cmd);
        swap_bytes!(self, cmdsize);
        swap_bytes!(self, vmaddr);
        swap_bytes!(self, vmsize);
        swap_bytes!(self, fileoff);
        swap_bytes!(self, filesize);
        swap_bytes!(self, maxprot);
        swap_bytes!(self, initprot);
        swap_bytes!(self, nsects);
        swap_bytes!(self, flags);
    }
}

#[derive(Debug)]
pub struct LoadCommand {
    pub cmd: u32,
    pub cmdsize: u32,
}

impl LoadCommand {
    pub fn from(buffer: [u8; 8], is_little_endian: bool) -> LoadCommand {
        let lc_buffer: [u32; 2] = unsafe { std::mem::transmute_copy::<[u8; 8], [u32; 2]>(&buffer) };
        let mut load_command = LoadCommand {
            cmd: lc_buffer[0],
            cmdsize: lc_buffer[1],
        };

        if is_little_endian {
            load_command.fix_endian();
        }

        load_command
    }
}

impl FixLoadCommandEndian for LoadCommand {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cmd);
        swap_bytes!(self, cmdsize);
    }
}

#[derive(Debug)]
pub struct LinkeditDataCommand {
    pub cmd: u32,
    pub cmdsize: u32,
    pub dataoff: u32,
    pub datasize: u32,
}

impl LinkeditDataCommand {
    pub fn from(buffer: [u8; 16], is_little_endian: bool) -> LinkeditDataCommand {
        let ldc_buffer: [u32; 4] =
            unsafe { std::mem::transmute_copy::<[u8; 16], [u32; 4]>(&buffer) };
        let mut linkedit_data_command = LinkeditDataCommand {
            cmd: ldc_buffer[0],
            cmdsize: ldc_buffer[1],
            dataoff: ldc_buffer[2],
            datasize: ldc_buffer[3],
        };

        if is_little_endian {
            linkedit_data_command.fix_endian();
        }

        linkedit_data_command
    }
}

impl FixLoadCommandEndian for LinkeditDataCommand {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cmd);
        swap_bytes!(self, cmdsize);
        swap_bytes!(self, dataoff);
        swap_bytes!(self, datasize);
    }
}

#[derive(Debug)]
pub struct Dylib {
    pub name_offset: u32,
    pub timestamp: u32,
    pub current_version: u32,
    pub compatibility_version: u32,
}

impl FixLoadCommandEndian for Dylib {
    fn fix_endian(&mut self) {
        swap_bytes!(self, name_offset);
        swap_bytes!(self, timestamp);
        swap_bytes!(self, current_version);
        swap_bytes!(self, compatibility_version);
    }
}

#[derive(Debug)]
pub struct DylibCommand {
    pub cmd: u32,
    pub cmdsize: u32,
    pub dylib: Dylib,
}

impl DylibCommand {
    pub fn default() -> DylibCommand {
        DylibCommand {
            cmd: 0,
            cmdsize: 0,
            dylib: Dylib {
                name_offset: 0,
                timestamp: 0,
                current_version: 0,
                compatibility_version: 0,
            },
        }
    }

    pub fn from(buffer: [u8; 24], is_little_endian: bool) -> DylibCommand {
        let dc_buffer: [u32; 6] =
            unsafe { std::mem::transmute_copy::<[u8; 24], [u32; 6]>(&buffer) };
        let mut dylib_command = DylibCommand {
            cmd: dc_buffer[0],
            cmdsize: dc_buffer[1],
            dylib: Dylib {
                name_offset: dc_buffer[2],
                timestamp: dc_buffer[3],
                current_version: dc_buffer[4],
                compatibility_version: dc_buffer[5],
            },
        };

        if is_little_endian {
            dylib_command.fix_endian();
        }

        dylib_command
    }

    pub fn to_u8(&self) -> [u8; 24] {
        let mut data: [u32; 6] = [0u32; 6];
        data[0] = self.cmd;
        data[1] = self.cmdsize;
        data[2] = self.dylib.name_offset;
        data[3] = self.dylib.timestamp;
        data[4] = self.dylib.current_version;
        data[5] = self.dylib.compatibility_version;

        unsafe { std::mem::transmute_copy::<[u32; 6], [u8; 24]>(&data) }
    }

    pub fn len() -> u64 {
        24
    }
}

impl FixLoadCommandEndian for DylibCommand {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cmd);
        swap_bytes!(self, cmdsize);
        self.dylib.fix_endian();
    }
}

#[derive(Debug)]
pub struct SymtabCommand {
    pub cmd: u32,
    pub cmdsize: u32,
    pub symoff: u32,
    pub nsyms: u32,
    pub stroff: u32,
    pub strsize: u32,
}

impl SymtabCommand {
    pub fn from(buffer: [u8; 24], is_little_endian: bool) -> SymtabCommand {
        let sc_buffer: [u32; 6] =
            unsafe { std::mem::transmute_copy::<[u8; 24], [u32; 6]>(&buffer) };
        let mut symtab_command = SymtabCommand {
            cmd: sc_buffer[0],
            cmdsize: sc_buffer[1],
            symoff: sc_buffer[2],
            nsyms: sc_buffer[3],
            stroff: sc_buffer[4],
            strsize: sc_buffer[5],
        };

        if is_little_endian {
            symtab_command.fix_endian();
        }

        symtab_command
    }

    pub fn to_u8(&self) -> [u8; 24] {
        let mut data: [u32; 6] = [0u32; 6];
        data[0] = self.cmd;
        data[1] = self.cmdsize;
        data[2] = self.symoff;
        data[3] = self.nsyms;
        data[4] = self.stroff;
        data[5] = self.strsize;

        unsafe { std::mem::transmute_copy::<[u32; 6], [u8; 24]>(&data) }
    }
}

impl FixLoadCommandEndian for SymtabCommand {
    fn fix_endian(&mut self) {
        swap_bytes!(self, cmd);
        swap_bytes!(self, cmdsize);
        swap_bytes!(self, symoff);
        swap_bytes!(self, nsyms);
        swap_bytes!(self, stroff);
        swap_bytes!(self, strsize);
    }
}
