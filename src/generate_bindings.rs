use core::time;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::{env, fs, panic, thread};

fn find_include_dirs(
    path: &Path,
    ret: &mut Vec<String>,
    depth: usize,
) -> Result<(), std::io::Error> {
    if depth > 3 {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.file_name().unwrap() == "include" {
            ret.push(format!(
                "-I{}",
                fs::canonicalize(path.to_str().unwrap())
                    .unwrap()
                    .to_str()
                    .unwrap()
            ));
        } else if path.is_dir() {
            find_include_dirs(path.as_path(), ret, depth + 1)?;
        }
    }

    Ok(())
}
fn get_idf_path() -> String {
    match env::var("IDF_PATH") {
        Ok(v) => v,
        Err(_e) => {
            println!("Environment var IDF_PATH must be set. If you followed the the install instructions exactly, it would look like: '~/esp/esp-idf/'");
            std::process::exit(1);
        }
    }
}
fn get_idf_tools_path() -> String {
    let test_binary = "xtensa-esp32-elf-ar";
    let out = Command::new("which").arg(test_binary).output().expect("Unable to use which to locate the xtensa linker");
    if out.status.code().unwrap() != 0 {
        println!("Unable to find {} using which. This is needed to determine the ESP tools location.", test_binary);
        exit(1);
    }
    let tools_dir_str = String::from_utf8(out.stdout).unwrap();

    let mut tools_dir = PathBuf::from(tools_dir_str);
    for _ in 0..2 {
        tools_dir = tools_dir.parent().expect("Parent folder").to_path_buf();
    }

    return String::from(tools_dir.to_str().unwrap());
}

fn generate_bindings(idf_project_path:&String, src_h_path:&String, dest_rs_path:&String, prefix:&String) {
    let idf_path =get_idf_path();
    let idf_tools_path = get_idf_tools_path();

    // let llvm_config_path = "~/.xtensa/llvm_build/bin/llvm-config";
    // env::set_var("LLVM_CONFIG_PATH", llvm_config_path);
    // env::set_var("CLANG_PATH", llvm_config_path);
    // // env::set_var("RUST_LOG", "DEBUG");
    // env::set_var("RUST_LOG", "ERROR");
    // Path of xtensa-esp32-elf-ar is where the esp tools can be found.

    let mut clang_include_args: Vec<String> = Vec::new();

    clang_include_args.push(format!("--sysroot={}/xtensa-esp32-elf",idf_tools_path));
    clang_include_args.push("-D__bindgen".to_string());

    clang_include_args.push("-target".to_string());
    clang_include_args.push("xtensa".to_string());

    clang_include_args.push("-x".to_string());
    clang_include_args.push("c".to_string());

    clang_include_args.push(format!("-I{}/include",idf_tools_path)); // There is always one...
    clang_include_args.push(format!("-I{}/xtensa-esp32-elf/include",idf_tools_path)); // There is always one...
    clang_include_args.push(format!("-I{}/build/config", idf_project_path)); // There is always one...

    let components_path = format!("{}/components", idf_path);
    let search_includes = Path::new(components_path.as_str());
    find_include_dirs(search_includes, &mut clang_include_args, 0)
        .expect("Unable to scan for include directories.");

    let bindings = bindgen::Builder::default()
        .header(src_h_path)
        // .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .use_core()
        .ctypes_prefix(prefix)
        .layout_tests(false)
        .clang_args(clang_include_args.iter());

    let builder_result = panic::catch_unwind(|| {
        match bindings.generate() {
            Ok(x) => {
                println!("Saving bindings to file.");
                x.write_to_file(dest_rs_path)
                    .expect("Unable to write bindings to file src/bindings.rs");

                println!("Formatting rust source files.");
                Command::new("cargo")
                    .arg("fmt")
                    .status()
                    .expect("failed to run cargo format");

                std::process::exit(0);
            }
            Err(x) => {
                println!("Failed to parse: {:?}", x);
                // let mut output = String::new();
                // buf.read_to_string(&mut output).unwrap();
                // print!("captured: {}", output);

                std::process::exit(1);
            }
        }
    });

    match builder_result {
        Ok(_x) => {}
        Err(_x) => {
            let ten_millis = time::Duration::from_millis(1000);
            thread::sleep(ten_millis);
            eprintln!("Because the output from the sub-process is suppressed, no good message is available now. If you run:");
            eprintln!("{}", std::env::current_exe().unwrap().to_str().unwrap());
            std::process::exit(2);
        }
    };
    std::process::exit(0);
}

fn should_build(source_path: &String, target_path: &String) -> Result<bool, std::io::Error> {
    if !Path::new(source_path).exists() {
        return Err(std::io::Error::new(
            ErrorKind::Other,
            format!("missing {}",source_path)
        ));
    }

    if !Path::new(target_path).exists() {
        return Ok(true);
    }
    let src = fs::metadata(source_path)?.modified()?;
    let target = fs::metadata(target_path)?.modified()?;

    if src > target {
        return Ok(true);
    }

    Ok(false)
}

pub fn generate_bindings_from_build_rs(idf_project_path:&String, source_path:&String, target_path:&String, prefix:&String) {
    println!("BEGIN generate_bindings_from_build_rs: {:?}",std::env::current_exe().unwrap());
    if env::var("CARGO_MANIFEST_DIR").is_err() {
        env::set_var("CARGO_MANIFEST_DIR", env::current_dir().unwrap().to_str().unwrap());
    }
    println!("  CARGO_MANIFEST_DIR = {:?}", env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("  CWD = {:?}", env::current_dir().unwrap());

    println!("cargo:rerun-if-changed=src/bindings.h");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ESP_IDF");

    let should_build = should_build(&source_path, &target_path);
    if should_build.is_ok() {
        if should_build.unwrap_or(false) {
            generate_bindings(&idf_project_path,&source_path, &target_path, prefix);
        }
    } else {
        println!("Unable to build: {} relative to my path {:?}", should_build.unwrap_err(), env::current_dir().unwrap());

        std::process::exit(2);
    }
}
