// os/build.rs
/// generate src/link_app.S
/// run "xxx/user$ make build" first
/// user bin in ../user/src/bin/xxx.rs
/// compiled to ../user/target/riscv64gc-unknown-none-elf/release/xxx(.bin)

use std::fs::{File, read_dir};
use std::io::{Write, Result};

const USER_BIN_DIR: &str = "../user/src/bin";
const TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";
const OUT_LINK_APP: &str = "src/link_app.s";

// enumerate all files, get names in sort
fn list_apps() -> Vec<String> {
    let mut apps: Vec<String> = Vec::new();

    let dir = match read_dir(USER_BIN_DIR) {
        Ok(dir) => dir,
        Err(err) => panic!("Error reading directory: {}", err)
    };

    for entry in dir {
        // translate filename to string
        if let Ok(entry) = entry {
            if let Ok(name) = entry.file_name().into_string() {
                // only keep ".rs" files
                if name.ends_with(".rs") && name.len() > 3 {
                    let app_name = &name[0..name.len()-3]; // clip ".rs"
                    apps.push(app_name.to_string());
                }
            }
        }
    }

    apps.sort();
    apps
}

fn insert_app_data() -> Result<()> {
    let apps = list_apps();
    let mut file = File::create(OUT_LINK_APP)?; // add "?" for auto error handling

    // head
    writeln!(
        file,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(file, "    .quad app_{}_start", i)?;
    }
    // 单独列出最后一个程序的 end
    writeln!(file, "    .quad app_{}_end", apps.len() - 1)?;

    // incbin
    /*
    .section .data
    .global app_${idx}_start
    .global app_${idx}_end
    app_${idx}_start:
        .incbin "BIN_PATH"
    app_${idx}_end:
     */
    for idx in 0..apps.len() {
        let app = &apps[idx];
        writeln!(
            file,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{1}{2}.bin"
app_{0}_end:"#,
            idx, TARGET_PATH, app
        )?;
    }
    Ok(())
}


fn main() {
    // if file in USER_BIN_DIR changed, recompile
    println!("cargo:rerun-if-changed={}", USER_BIN_DIR);
    println!("cargo:rerun-if-changed={}", TARGET_PATH);

    insert_app_data().unwrap();
}