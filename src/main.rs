use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;

mod extensions;
mod macho;
mod opts;
mod utils;

use extensions::InsertDylibFileExt;
use macho::macho::*;
use macho::prelude::*;
use opts::Opts;
use utils::*;

fn main() -> std::io::Result<()> {
    let options = parse_arg();
    let lc_name = match options.weak {
        true => "LC_LOAD_WEAK_DYLIB",
        false => "LC_LOAD_DYLIB",
    };

    std::fs::copy(&options.binary_path, &options.output_path)?;
    let mut binary_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&options.output_path)?;
    let mut filesize = binary_file.metadata()?.len();

    let mut magic_buffer = [0; 4];
    binary_file.read_exact(&mut magic_buffer)?;

    let magic_buffer = unsafe { std::mem::transmute::<[u8; 4], u32>(magic_buffer) };

    match magic_buffer {
        MH_CIGAM_64 | MH_MAGIC_64 | MH_CIGAM | MH_MAGIC => {
            if insert_dylib(&mut binary_file, 0, &options, &mut filesize)? {
                binary_file.set_len(filesize as u64)?;
                println!("Added {} to {}", lc_name, options.binary_path);
            } else {
                println!("Failed to add {}", lc_name);
            }
        }
        FAT_MAGIC | FAT_CIGAM => {
            let is_little_endian = magic_buffer == FAT_CIGAM;
            binary_file.seek(SeekFrom::Start(0))?;

            let mut fat_header_buffer = [0u8; 8];
            binary_file.read_exact(&mut fat_header_buffer)?;
            let mut fh = FatHeader::from(fat_header_buffer, is_little_endian);

            let nfat_arch = fh.nfat_arch as usize;
            println!("Binary is a fat binary with {} archs.", nfat_arch);

            let mut archs: Vec<FatArch> = Vec::new();
            for _arch_index in 0..nfat_arch {
                let mut arch_buffer = [0u8; 20];
                binary_file.read_exact(&mut arch_buffer)?;
                archs.push(FatArch::from(arch_buffer, is_little_endian));
            }

            let mut fails = 0usize;
            let mut offset: u64 = 0;
            if nfat_arch > 0 {
                offset = archs[0].offset as u64;
                if !is_little_endian {
                    offset = offset.swap_bytes();
                }
            }

            let _ = archs
                .iter_mut()
                .enumerate()
                .map(|(arch_index, current_arch)| -> io::Result<()> {
                    let orig_offset = current_arch.offset as u64;
                    let orig_slice_size = current_arch.size as u64;

                    let mut align: u32 = 1 << current_arch.align;
                    if !is_little_endian {
                        align = align.swap_bytes();
                    }

                    offset = round_up_u64(offset, align as u64);

                    if orig_offset != offset {
                        binary_file.fmemmove(offset, orig_offset, orig_slice_size)?;
                        let diff = (offset as i64 - orig_offset as i64).abs() as u64;
                        binary_file.fbzero(min(offset, orig_offset) + orig_slice_size, diff)?;
                        current_arch.offset = offset as u32;
                    }

                    let mut slice_size = orig_slice_size;
                    let ret = insert_dylib(&mut binary_file, offset, &options, &mut slice_size)?;
                    if !ret {
                        println!("Failed to add {} to arch #{}", lc_name, arch_index + 1);
                        fails += 1;
                    }

                    if slice_size < orig_slice_size && arch_index < nfat_arch - 1 {
                        binary_file.fbzero(offset + slice_size, orig_slice_size - slice_size)?;
                    }

                    filesize = offset + slice_size;
                    offset += slice_size;
                    current_arch.size = slice_size as u32;

                    Ok(())
                })
                .collect::<Vec<io::Result<()>>>();

            binary_file.seek(SeekFrom::Start(0))?;
            if is_little_endian {
                fh.fix_endian();
            }
            binary_file.write_all(&fh.to_u8())?;

            let _ = archs
                .iter_mut()
                .map(|current_arch| -> io::Result<()> {
                    if is_little_endian {
                        current_arch.fix_endian();
                    }

                    binary_file.write_all(&current_arch.to_u8())?;
                    Ok(())
                })
                .collect::<Vec<io::Result<()>>>();

            binary_file.set_len(filesize as u64)?;
            if fails == 0 {
                println!("Added {} to all archs in {}", lc_name, options.binary_path);
            } else if fails != nfat_arch {
                println!(
                    "Added {} to {}/{} archs in {}",
                    lc_name,
                    nfat_arch - fails,
                    nfat_arch,
                    options.binary_path
                )
            } else {
                println!("Failed to add {} to any archs.", lc_name)
            }
        }
        _ => {
            println!("Not a MachO binary: {}", options.binary_path);
        }
    }

    Ok(())
}

fn insert_dylib(
    mut binary_file: &mut File,
    header_offset: u64,
    options: &Opts,
    slice_size: &mut u64,
) -> io::Result<bool> {
    binary_file.seek(SeekFrom::Start(header_offset))?;

    let mut header_buffer = [0u8; 32];
    binary_file.read_exact(&mut header_buffer)?;

    let mut mach_header = MachHeader::from(header_buffer);
    match mach_header.magic {
        MH_CIGAM_64 | MH_MAGIC_64 | MH_CIGAM | MH_MAGIC => (),
        _ => {
            println!("Unknown MachO header magic: {:08x}", mach_header.magic);
        }
    }

    let commands_offset = header_offset + mach_header.len();
    let cont = check_load_commands(
        &mut binary_file,
        &mut mach_header,
        header_offset,
        commands_offset,
        &options,
        slice_size,
    )?;
    if !cont {
        return Ok(true);
    }

    let path_padding = 8u32;
    let dylib_path_len = options.dylib_path.len() as u32;
    let dylib_path_size = (dylib_path_len & !(path_padding - 1)) + path_padding;
    let cmdsize: u32 = dylib_path_size + DylibCommand::len() as u32;

    let mut dylib_command = DylibCommand::default();
    dylib_command.cmd = match options.weak {
        true => LC_LOAD_WEAK_DYLIB,
        false => LC_LOAD_DYLIB,
    };
    dylib_command.cmdsize = cmdsize;
    dylib_command.dylib.name_offset = DylibCommand::len() as u32;
    dylib_command.dylib.timestamp = 0;
    dylib_command.dylib.current_version = 0;
    dylib_command.dylib.compatibility_version = 0;

    let mut sizeofcmds = mach_header.sizeofcmds;
    if MachHeader::is_little_endian(mach_header.magic) {
        sizeofcmds = sizeofcmds.swap_bytes();
        dylib_command.fix_endian();
    }
    binary_file.seek(SeekFrom::Start(commands_offset + sizeofcmds as u64))?;

    let mut space: Vec<u8> = Vec::new();
    space.resize(cmdsize as usize, 0);
    binary_file.read_exact(&mut space[..])?;

    let mut empty = true;
    for i in 0..(cmdsize as usize) {
        if space[i] != 0 {
            empty = false;
            break;
        }
    }

    if !empty {
        println!("It doesn't seem like there is enough empty space. Will continue though...");
    }

    binary_file.seek(SeekFrom::Current(0 - cmdsize as i64))?;
    binary_file.write_all(&dylib_command.to_u8())?;
    binary_file.write_all(options.dylib_path.as_ref())?;

    sizeofcmds += cmdsize;

    if MachHeader::is_little_endian(mach_header.magic) {
        mach_header.ncmds = (mach_header.ncmds.swap_bytes() + 1).swap_bytes();
        mach_header.sizeofcmds = sizeofcmds.swap_bytes();
    } else {
        mach_header.ncmds = mach_header.ncmds + 1;
        mach_header.sizeofcmds = sizeofcmds;
    }

    binary_file.seek(SeekFrom::Start(header_offset))?;
    binary_file.write_all(&mach_header.to_u8())?;

    Ok(true)
}

fn check_load_commands(
    binary_file: &mut File,
    mach_header: &mut MachHeader,
    header_offset: u64,
    commands_offset: u64,
    options: &Opts,
    slice_size: &mut u64,
) -> io::Result<bool> {
    binary_file.seek(SeekFrom::Start(commands_offset))?;

    let is_little_endian = MachHeader::is_little_endian(mach_header.magic);

    let ncmds = mach_header.ncmds;

    let mut linkedit_32_pos = -1i64;
    let mut linkedit_64_pos = -1i64;
    let mut linkedit_32 = SegmentCommand::default();
    let mut linkedit_64 = SegmentCommand64::default();

    let mut symtab_pos = -1i64;

    let mut skip_fbzero_before_fix_header = false;
    for i in 0..ncmds {
        let mut load_command_buffer = [0u8; 8];
        binary_file.fpeek(&mut load_command_buffer)?;

        let lc = LoadCommand::from(load_command_buffer, is_little_endian);

        match lc.cmd {
            LC_CODE_SIGNATURE => {
                if i == ncmds - 1 {
                    if !options.strip_codesign {
                        return Ok(true);
                    }

                    let mut linkedit_data_command_buffer = [0u8; 16];
                    binary_file.fpeek(&mut linkedit_data_command_buffer)?;

                    let cmd =
                        LinkeditDataCommand::from(linkedit_data_command_buffer, is_little_endian);

                    let current_offset = binary_file.ftello();
                    binary_file.fbzero(current_offset, lc.cmdsize as u64)?;

                    let mut linkedit_fileoff = 0u64;
                    let mut linkedit_filesize = 0u64;

                    if linkedit_32_pos != -1 {
                        linkedit_fileoff = linkedit_32.fileoff as u64;
                        linkedit_filesize = linkedit_32.filesize as u64;
                    } else if linkedit_64_pos != -1 {
                        linkedit_fileoff = linkedit_64.fileoff;
                        linkedit_filesize = linkedit_64.filesize;
                    } else {
                        println!("Warning: __LINKEDIT segment not found.");
                    }

                    if linkedit_32_pos != -1 || linkedit_64_pos != -1 {
                        if is_little_endian {
                            linkedit_fileoff = linkedit_fileoff.swap_bytes();
                            linkedit_filesize = linkedit_filesize.swap_bytes();
                        }

                        if linkedit_fileoff + linkedit_filesize != *slice_size {
                            println!("Warning: __LINKEDIT segment is not at the end of the file, so codesign will not work on the patched binary.");
                        } else {
                            if (cmd.dataoff + cmd.datasize) as u64 != *slice_size {
                                println!("Warning: Codesignature is not at the end of __LINKEDIT segment, so codesign will not work on the patched binary.");
                            } else {
                                *slice_size -= cmd.datasize as u64;

                                if symtab_pos == -1 {
                                    println!("Warning: LC_SYMTAB load command not found. codesign might not work on the patched binary.");
                                } else {
                                    binary_file.seek(SeekFrom::Start(symtab_pos as u64))?;
                                    let mut symtab_command_buffer = [0u8; 24];
                                    binary_file.fpeek(&mut symtab_command_buffer)?;
                                    let mut symtab = SymtabCommand::from(
                                        symtab_command_buffer,
                                        is_little_endian,
                                    );
                                    let diffsize = (symtab.stroff + symtab.strsize) as i64
                                        - (*slice_size as i64);
                                    if -16 <= diffsize && diffsize <= 0 {
                                        symtab.strsize =
                                            ((symtab.strsize as i32) - (diffsize as i32)) as u32;
                                        if is_little_endian {
                                            symtab.strsize = symtab.strsize.swap_bytes();
                                        }

                                        binary_file.write_all(&symtab.to_u8())?;
                                    } else {
                                        println!("Warning: String table doesn't appear right before code signature. codesign might not work on the patched binary. {:016x}", diffsize);
                                    }
                                }

                                linkedit_filesize -= cmd.datasize as u64;
                                let linkedit_vmsize = round_up_u64(linkedit_filesize, 0x1000);

                                if linkedit_32_pos != -1 {
                                    linkedit_32.filesize = linkedit_filesize as u32;
                                    linkedit_32.vmsize = linkedit_vmsize as u32;

                                    if is_little_endian {
                                        linkedit_32.filesize = linkedit_32.filesize.swap_bytes();
                                        linkedit_32.vmsize = linkedit_32.vmsize.swap_bytes();
                                    }

                                    binary_file.seek(SeekFrom::Start(linkedit_32_pos as u64))?;
                                    binary_file.write_all(&linkedit_32.to_u8())?;
                                } else {
                                    linkedit_64.filesize = linkedit_filesize;
                                    linkedit_64.vmsize = linkedit_vmsize;

                                    if is_little_endian {
                                        linkedit_64.filesize = linkedit_64.filesize.swap_bytes();
                                        linkedit_64.vmsize = linkedit_64.vmsize.swap_bytes();
                                    }

                                    binary_file.seek(SeekFrom::Start(linkedit_64_pos as u64))?;
                                    binary_file.write_all(&linkedit_64.to_u8())?;
                                }

                                skip_fbzero_before_fix_header = true;
                            }
                        }
                    }

                    if !skip_fbzero_before_fix_header {
                        binary_file
                            .fbzero(header_offset + cmd.dataoff as u64, cmd.datasize as u64)?;
                    }

                    let new_sizeofcmds = mach_header.sizeofcmds - lc.cmdsize;
                    fix_header(mach_header, ncmds - 1, new_sizeofcmds);
                } else {
                    println!("LC_CODE_SIGNATURE is not the last load command, so couldn't remove.");
                }
            }
            LC_LOAD_DYLIB | LC_LOAD_WEAK_DYLIB => {
                let mut dylib_command_buffer = [0u8; 24];
                binary_file.fpeek(&mut dylib_command_buffer)?;
                let dylib_command = DylibCommand::from(dylib_command_buffer, is_little_endian);

                let mut dylib_name_buffer: Vec<u8> = Vec::with_capacity(lc.cmdsize as usize);
                dylib_name_buffer.resize(lc.cmdsize as usize, 0);
                binary_file.fpeek(&mut dylib_name_buffer)?;

                let dylib_name_start: usize = dylib_command.dylib.name_offset as usize;
                let dylib_name_max_index = lc.cmdsize as usize;
                let mut dylib_name_end = 0;

                for index in dylib_name_start..dylib_name_max_index {
                    if dylib_name_buffer[index] == 0 {
                        dylib_name_end = index;
                        break;
                    }
                }
                let name = match String::from_utf8(
                    dylib_name_buffer[dylib_name_start..dylib_name_end].to_vec(),
                ) {
                    Ok(name) => name,
                    Err(e) => {
                        println!("Cannot get dylib path for load command at {}: {}", i, e);
                        continue;
                    }
                };

                if name.eq(&options.dylib_path) {
                    println!("Binary already contains a load command for that dylib.");
                    return Ok(true);
                }
            }
            LC_SEGMENT | LC_SEGMENT_64 => {
                if lc.cmd == LC_SEGMENT {
                    let mut segment_command_buffer = [0u8; 56];
                    binary_file.fpeek(&mut segment_command_buffer)?;
                    let cmd = SegmentCommand::from(segment_command_buffer, is_little_endian);
                    if get_segname(&cmd.segname).eq("__LINKEDIT".into()) {
                        linkedit_32_pos = binary_file.ftello() as i64;
                        linkedit_32 = cmd;
                    }
                } else {
                    let mut segment_command_buffer = [0u8; 72];
                    binary_file.fpeek(&mut segment_command_buffer)?;
                    let cmd = SegmentCommand64::from(segment_command_buffer, is_little_endian);
                    if get_segname(&cmd.segname).eq("__LINKEDIT".into()) {
                        linkedit_64_pos = binary_file.ftello() as i64;
                        linkedit_64 = cmd;
                    }
                }
            }
            LC_SYMTAB => {
                symtab_pos = binary_file.ftello() as i64;
            }
            _ => (),
        }
        binary_file.seek(SeekFrom::Current(lc.cmdsize as i64))?;
    }

    Ok(true)
}

fn fix_header(mach_header: &mut MachHeader, ncmds: u32, sizeofcmds: u32) {
    mach_header.ncmds = ncmds;
    mach_header.sizeofcmds = sizeofcmds;
}
