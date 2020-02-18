#[derive(Debug)]
pub struct Opts {
    pub dylib_path: String,
    pub binary_path: String,
    pub output_path: String,
    pub weak: bool,
    pub overwrite: bool,
    pub strip_codesign: bool,
    pub all_yes: bool,
}

impl Opts {
    pub fn default() -> Opts {
        Opts {
            dylib_path: "".to_string(),
            binary_path: "".to_string(),
            output_path: "".to_string(),
            weak: false,
            overwrite: false,
            strip_codesign: false,
            all_yes: false,
        }
    }
}
