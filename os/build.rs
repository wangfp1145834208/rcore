use std::fs::{read_dir, File};
use std::io::{Result, Write};

static TARGET_DIR: &'static str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn main() {
    insert_app_data().unwrap();
}

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.S")?;
    let mut apps = read_dir("../user/src/bin")?.
        into_iter().
        filter_map(|dir_entry| {
            let dir_entry = dir_entry.unwrap();
            if dir_entry.file_type().unwrap().is_dir() {
                return None;
            }
            let name_with_ext = dir_entry.file_name().into_string().unwrap();
            let mut it = name_with_ext.split('.');
            Some(it.next().unwrap_or_default().to_owned())
        }).
        collect::<Vec<_>>();
    apps.sort();

    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
        writeln!(f, r#"    .quad app_{}_name"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    writeln!(f, "")?;
    for (i, app_name) in apps.iter().enumerate() {
        writeln!(
            f,
            r#"    .global app_{0}_name
app_{0}_name:
    .byte {1}
    .ascii "{2}""#,
            i, app_name.len(), app_name
        )?;
    } 

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
    .align 3
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#,
            idx, app, TARGET_DIR
        )?;
    }
    Ok(())
}
