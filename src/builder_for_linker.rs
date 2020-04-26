use std::path::Path;
use ex::{fs};
use std::fmt::Write;
use ex::fs::{copy, create_dir_all};
use std::ffi::OsStr;
use regex::{Regex};
use std::env;
use crate::linker_that_configures_cmake::linker_main;


pub fn install_as_linker_or_run_as_linker() {
    println!("BEGIN install_as_linker__or__run_as_linker: {:?}",std::env::current_exe().unwrap());
    if env::var("CARGO_MANIFEST_DIR").is_err() {
        env::set_var("CARGO_MANIFEST_DIR", env::current_dir().unwrap().to_str().unwrap());
    }
    println!("  CARGO_MANIFEST_DIR = {:?}", env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("  CWD = {:?}", env::current_dir().unwrap());

    println!("cargo:rerun-if-changed=sdkconfig");
    println!("cargo:rerun-if-changed=build.rs");

    let me = std::env::current_exe().unwrap();
    let me_ext = me.extension().unwrap_or( OsStr::new("")).to_str().unwrap();
    let linker = format!("esp-rust-linker{}{}",
                         if me_ext.len() > 0 {"."} else {""},
                         me_ext);
    let linker_path = format!("{}/target/{}", env::var("CARGO_MANIFEST_DIR").unwrap(), linker);

    let running_as_linker = me.file_name().unwrap().eq(linker.as_str());

    if running_as_linker {
        linker_main().expect("Failed to configure cmake");
    } else {
        install_linker(&linker_path);
    }
}

fn install_linker(linker_path: &String) {
    println!("BEGIN install_linker: {:?}",std::env::current_exe().unwrap());

    let cargo_config_path = Path::new(".cargo/config");
    let linker_line = format!("linker = \"{}\"", linker_path);
    if cargo_config_path.exists() {
        let cargo_config = fs::read_to_string(cargo_config_path).unwrap();

        let re = Regex::new("linker[ ]*=[^\n]*").unwrap();
        if re.is_match(&cargo_config) {
            let new_cargo_config = re.replace_all(cargo_config.as_str(), linker_line.as_str()).to_string();

            if !cargo_config.eq(&new_cargo_config) {
                fs::write(cargo_config_path, &new_cargo_config).unwrap();
            }
        } else {
            fs::write(cargo_config_path, format!("{}\n[target.xtensa-esp32-none-elf]\n{}", cargo_config, linker_line)).unwrap();
        }
    } else {
        create_dir_all(cargo_config_path.parent().unwrap()).unwrap();
        let mut cargo_config = String::new();
        writeln!(cargo_config, "[build]").unwrap();
        writeln!(cargo_config, "target = \"xtensa-esp32-none-elf\"").unwrap();
        writeln!(cargo_config, "[target.xtensa-esp32-none-elf]").unwrap();
        writeln!(cargo_config, "{}", linker_line).unwrap();
        fs::write(cargo_config_path, cargo_config).unwrap();
    }
// Copy this to the target folder.
    let curr_exe = std::env::current_exe().unwrap();
    let src = Path::new(&curr_exe);
    let dst = Path::new(&linker_path);
    if src.exists() && (!dst.exists() || fs::metadata(src).unwrap().modified().unwrap() > fs::metadata(src).unwrap().modified().unwrap()) {
        create_dir_all(dst.parent().unwrap().to_str().unwrap()).unwrap();

        copy(src, dst).unwrap_or_else(|e| { panic!("Unable to copy {:?} to {:?} relative to my path of {:?}:\n{}", src, dst, env::current_dir().unwrap(), e); });
    }
}