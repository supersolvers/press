// SPDX-License-Identifier: GPL-3.0-or-later
//! A style from outside: the one in styles/ninetyfive is built as a library
//! of its own, as anybody's would be, and a window is drawn wearing it.

use press::abi::*;

const OK: i32 = Status::Ok as i32;

/// Build the style with the system's C compiler. None on a machine without one.
fn build(out: &std::path::Path) -> Option<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lib = out.join("libpress_style_ninetyfive.so");
    let done = std::process::Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-Wall", "-Werror"])
        .arg("-I")
        .arg(root.join("include"))
        .arg(root.join("styles/ninetyfive/ninetyfive.c"))
        .arg("-o")
        .arg(&lib)
        .status()
        .ok()?;
    assert!(done.success(), "the style does not compile against press_style.h");
    Some(lib)
}

fn pixels(path: &std::path::Path) -> (png::ColorType, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap())).read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut data).unwrap();
    data.truncate(frame.buffer_size());
    (frame.color_type, data)
}

#[test]
fn a_window_wears_a_style_from_a_library_of_its_own() {
    let dir = std::env::temp_dir().join(format!("press-style-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let Some(lib) = build(&dir) else {
        eprintln!("no C compiler: the style is not tried");
        return;
    };
    // this file is a process of its own, so the choice is nobody else's
    std::env::set_var("PRESS_STYLE", &lib);
    std::env::set_var("PRESS_BACKEND", "headless");
    unsafe {
        assert_eq!(press_app_init(), OK);
        let (mut window, mut button) = (0u64, 0u64);
        assert_eq!(press_window_new("Style".as_ptr(), 5, 160, 60, &mut window), OK);
        assert_eq!(press_button_new("Press".as_ptr(), 5, &mut button), OK);
        assert_eq!(press_window_set_content(window, button), OK);
        let picture = dir.join("worn.png");
        let name = picture.to_str().unwrap();
        assert_eq!(press_window_snapshot(window, name.as_ptr(), name.len(), 1), OK);
        let (kind, data) = pixels(&picture);
        // grey, which black and white has none of: the window is the style's
        assert_eq!(kind, png::ColorType::Rgb);
        let grey = data.chunks(3).filter(|p| *p == [0xc0, 0xc0, 0xc0]).count();
        assert!(grey > 160 * 60 / 2, "most of the window is the style's grey, not {grey} pixels of it");
        // and the words on the button are still the engine's, in black
        assert!(data.chunks(3).any(|p| p == [0, 0, 0]));
        assert_eq!(press_app_shutdown(), OK);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
