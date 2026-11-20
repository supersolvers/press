// SPDX-License-Identifier: GPL-3.0-or-later
//! A style from outside: the one in styles/ninetyfive is built as a library
//! of its own, as anybody's would be, and a window is drawn wearing it.

use press::abi::*;

const OK: i32 = Status::Ok as i32;

/// Build a style with the system's C compiler. None on a machine without one.
fn build(name: &str, out: &std::path::Path) -> Option<std::path::PathBuf> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lib = out.join(format!("libpress_style_{name}.so"));
    let done = std::process::Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-I")
        .arg(root.join("include"))
        .arg("-I")
        .arg(root.join("styles/common"))
        .arg(root.join(format!("styles/{name}/{name}.c")))
        .arg("-lm")
        .arg("-o")
        .arg(&lib)
        .status()
        .ok()?;
    assert!(done.success(), "the style {name} does not compile against press_style.h");
    Some(lib)
}

fn pixels(path: &std::path::Path) -> (png::ColorType, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap())).read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut data).unwrap();
    data.truncate(frame.buffer_size());
    (frame.color_type, data)
}

/// Draw a small dialog and say what its picture is made of: the kind of
/// picture, and its pixels as red, green and blue.
unsafe fn dialog(dir: &std::path::Path, name: &str) -> (png::ColorType, Vec<u8>) {
    let (mut window, mut column, mut label, mut button, mut list) = (0u64, 0u64, 0u64, 0u64, 0u64);
    assert_eq!(press_window_new("Style".as_ptr(), 5, 200, 160, &mut window), OK);
    assert_eq!(press_stack_new(Orientation::Vertical as i32, &mut column), OK);
    assert_eq!(press_label_new("Words".as_ptr(), 5, &mut label), OK);
    assert_eq!(press_button_new("Press".as_ptr(), 5, &mut button), OK);
    assert_eq!(press_list_box_new(&mut list), OK);
    assert_eq!(press_list_box_add(list, "A row".as_ptr(), 5), OK);
    for widget in [label, button, list] {
        assert_eq!(press_stack_add(column, widget), OK);
    }
    assert_eq!(press_window_set_content(window, column), OK);
    let picture = dir.join(format!("{name}.png"));
    let path = picture.to_str().unwrap();
    assert_eq!(press_window_snapshot(window, path.as_ptr(), path.len(), 1), OK);
    assert_eq!(press_widget_destroy(window), OK);
    pixels(&picture)
}

/// Every style that comes with the library builds as a library of its own,
/// as anybody's would, and a window drawn wearing it is of its colour. One
/// process wears one style, so each is tried in a process of its own: this
/// test runs itself again for each, with the style named in the environment.
#[test]
fn windows_wear_styles_from_libraries_of_their_own() {
    let styles: [(&str, [u8; 3]); 13] = [
        ("ninetyone", [0xdd, 0xdd, 0xdd]),
        ("eightyeight", [0xcc, 0xcc, 0xcc]),
        ("ninetythree", [0xae, 0xb2, 0xc3]),
        ("ninetyeight", [0xd8, 0xd8, 0xd8]),
        ("eightyfive", [0x00, 0x55, 0xaa]),
        ("eightynine", [0xaa, 0xaa, 0xaa]),
        ("ninety", [0xff, 0xff, 0xff]),
        ("twothousandfour", [0xef, 0xeb, 0xe7]),
        ("ninetyfive", [0xc0, 0xc0, 0xc0]),
        ("ninetynine", [0xdd, 0xdd, 0xdd]),
        ("twothousand", [0xd4, 0xd0, 0xc8]),
        ("twothousandone", [0xec, 0xe9, 0xd8]),
        ("twothousandfive", [0xec, 0xec, 0xec]),
    ];
    if let Ok(wearing) = std::env::var("PRESS_STYLE_UNDER_TEST") {
        let (name, colour) = styles.iter().find(|(name, _)| wearing.ends_with(&format!("libpress_style_{name}.so"))).expect("a style this test knows");
        let dir = std::path::Path::new(&wearing).parent().unwrap().to_path_buf();
        unsafe {
            assert_eq!(press_app_init(), OK);
            let (kind, data) = dialog(&dir, name);
            // a colour black and white has none of: the window is the style's
            assert_eq!(kind, png::ColorType::Rgb, "{name}");
            let own = data.chunks(3).filter(|p| *p == colour).count();
            assert!(own > 200 * 160 / 8, "{name}: much of the window is the style's colour, not {own} pixels of it");
            // the paper of the list is white whatever the window is (but in
            // the styles whose paper is the window's own colour, or cream)
            if !["eightyfive", "eightyeight", "ninetythree"].contains(name) {
                assert!(data.chunks(3).filter(|p| *p == [0xff, 0xff, 0xff]).count() > 500, "{name}");
            }
            // and the window has a frame of its own round it
            assert!(data.len() > 200 * 160 * 3, "{name}");
            assert_eq!(press_app_shutdown(), OK);
        }
        return;
    }
    let dir = std::env::temp_dir().join(format!("press-style-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, _) in styles {
        let Some(lib) = build(name, &dir) else {
            eprintln!("no C compiler: the styles are not tried");
            return;
        };
        let again = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "windows_wear_styles_from_libraries_of_their_own", "--nocapture"])
            .env("PRESS_STYLE", &lib)
            .env("PRESS_STYLE_UNDER_TEST", &lib)
            .env("PRESS_BACKEND", "headless")
            .status()
            .unwrap();
        assert!(again.success(), "a window does not wear {name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
