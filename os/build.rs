use std::fs::{File, read_dir};
use std::io::{Result, Write};

static TARGET_DIR: &'static str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn main() {
    insert_app_data().unwrap();
}

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.S")?;
    let mut apps = read_dir("../user/src/bin")?.
        into_iter().
        map(|dir_entry| {
            let name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            let mut it = name_with_ext.split('.');
            it.next().unwrap_or_default().to_owned()
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
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}{1}.bin"
app_{0}_end:"#,
            idx, app, TARGET_DIR
        )?;
    }
    Ok(())
}
