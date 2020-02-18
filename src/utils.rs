extern crate clap;
use crate::opts::Opts;
use clap::{App, Arg};

pub fn parse_arg() -> Opts {
    let matches = App::new("Insert Dylib")
        .version("0.1.0")
        .author("Cocoa <0xbbc@0xbbc.com>")
        .about("Insert dylib into Mach-O binary")
        .arg(
            Arg::with_name("dylib_path")
                .short("d")
                .long("dylib")
                .required(true)
                .help("dylib path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("binary_path")
                .short("b")
                .long("binary")
                .required(true)
                .help("binary file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .required(false)
                .help("output path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("weak")
                .long("weak")
                .multiple(false)
                .help("Sets LC_LOAD_WEAK_DYLIB"),
        )
        .arg(
            Arg::with_name("overwrite")
                .long("overwrite")
                .multiple(false)
                .help("Overwrite existent file"),
        )
        .arg(
            Arg::with_name("strip_codesign")
                .long("strip-codesign")
                .multiple(false)
                .help("Strip codesign"),
        )
        .arg(
            Arg::with_name("all_yes")
                .long("all-yes")
                .multiple(false)
                .help("Yes to all"),
        )
        .get_matches();

    let mut options = Opts::default();
    options.dylib_path = String::from(matches.value_of("dylib_path").unwrap());
    options.binary_path = matches.value_of("binary_path").unwrap().into();

    let mut default_output_path = String::new();
    default_output_path.push_str(&*options.binary_path);
    default_output_path.push_str("_patched");
    options.output_path = String::from(matches.value_of("OUTPUT").unwrap_or(&*default_output_path));

    options.weak = matches.occurrences_of("weak") == 1;
    options.overwrite = matches.occurrences_of("overwrite") == 1;
    options.strip_codesign = matches.occurrences_of("strip_codesign") == 1;
    options.all_yes = matches.occurrences_of("all-yes") == 1;
    options
}

pub fn round_up_u64(x: u64, y: u64) -> u64 {
    (((x) + (y) - 1) & (!y + 1))
}

pub fn get_segname(segname: &[u8; 16]) -> String {
    let mut name = String::new();
    for i in 0..segname.len() {
        let current_char = segname[i] as char;
        if current_char == '\0' {
            break;
        }
        name.push(current_char);
    }
    name
}
