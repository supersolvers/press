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
    let styles: [(&str, [u8; 3]); 14] = [
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
        ("twothousandtwentyseven", [0xee, 0xf3, 0xfa]),
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

/// A theme is a style in other colours, with no compiler: a file of text
/// beside the library, chosen by its own name. As above, the window is
/// drawn in a process of its own.
#[test]
fn a_theme_wears_a_style_in_other_colours() {
    if let Ok(dir) = std::env::var("PRESS_THEME_UNDER_TEST") {
        let dir = std::path::PathBuf::from(dir);
        unsafe {
            assert_eq!(press_app_init(), OK);
            let (_, data) = dialog(&dir, "mint");
            // the window is of the theme's colour, and none of it of the style's own
            let of = |colour: [u8; 3]| data.chunks(3).filter(|p| *p == colour).count();
            assert!(of([0xaa, 0xdd, 0xcc]) > 200 * 160 / 8, "much of the window is the theme's colour, not {} pixels of it", of([0xaa, 0xdd, 0xcc]));
            assert_eq!(of([0xc0, 0xc0, 0xc0]), 0);
            assert_eq!(press_app_shutdown(), OK);
        }
        return;
    }
    let dir = std::env::temp_dir().join(format!("press-theme-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    if build("ninetyfive", &dir).is_none() {
        eprintln!("no C compiler: the theme is not tried");
        return;
    }
    std::fs::write(dir.join("mint.toml"), "name = \"Mint\"\nstyle = \"ninetyfive\"\n[colours]\nface = \"#aaddcc\"\nno_such = \"#000000\"\n").unwrap();
    let again = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "a_theme_wears_a_style_in_other_colours", "--nocapture"])
        .env("PRESS_STYLE", "mint")
        .env("PRESS_STYLE_PATH", &dir)
        .env("PRESS_THEME_UNDER_TEST", &dir)
        .env("PRESS_BACKEND", "headless")
        .output()
        .unwrap();
    assert!(again.status.success(), "a window does not wear the theme: {}", String::from_utf8_lossy(&again.stderr));
    // what the theme asked for that the style has not is said, with what it has
    let said = String::from_utf8_lossy(&again.stderr);
    assert!(said.contains("no colour named no_such") && said.contains("face, hilite"), "{said}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The themes that come with the library are read without a doubt, and
/// each names only colours its style has.
#[test]
fn the_themes_that_come_with_it_are_sound() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut tried = 0;
    for entry in std::fs::read_dir(root.join("themes")).unwrap() {
        let path = entry.unwrap().path();
        let theme = press::theme::Theme::parse(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
        assert!(theme.doubts.is_empty(), "{}: {:?}", path.display(), theme.doubts);
        assert!(theme.name.is_some(), "{}", path.display());
        assert!(path.file_stem().unwrap().to_string_lossy().starts_with(&format!("{}-", theme.style)), "{} is named for its style", path.display());
        // (Two Thousand is Ninety-Five's source, told other colours)
        let source = if theme.style == "twothousand" { "ninetyfive".to_string() } else { theme.style.clone() };
        let source = std::fs::read_to_string(root.join(format!("styles/{source}/{source}.c"))).unwrap();
        for (name, _) in &theme.colours {
            assert!(source.contains(&format!("{{\"{name}\", &")), "{}: {} has no colour named {name}", path.display(), theme.style);
        }
        tried += 1;
    }
    assert!(tried >= 6);
}
