// SPDX-License-Identifier: GPL-3.0-or-later
//! The programs installed on the machine, as their makers describe them
//! in desktop entries: the `.desktop` files under `applications` in the
//! user's data folder and the system's.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// What it is called, for people to read.
    pub name: String,
    /// The program to run and what to give it.
    pub command: Vec<String>,
    /// What kind of program it is, in the words of the desktop entry
    /// specification (`Game`, `Graphics`, `Utility`, ...), as listed.
    pub categories: Vec<String>,
}

/// The sections programs are put in, by the main categories of the
/// desktop entry specification: the section's name, and the categories
/// that put a program in it.
pub const SECTIONS: [(&str, &[&str]); 11] = [
    ("Education", &["Education"]),
    ("Games", &["Game"]),
    ("Graphics", &["Graphics"]),
    ("Internet", &["Network"]),
    ("Office", &["Office"]),
    ("Programming", &["Development"]),
    ("Science", &["Science"]),
    ("Settings", &["Settings"]),
    ("Sound & Video", &["AudioVideo", "Audio", "Video"]),
    ("System", &["System"]),
    ("Utilities", &["Utility"]),
];

/// What a program's section is called when none of its categories puts
/// it in one.
pub const OTHER: &str = "Other";

impl Program {
    /// The section a program goes in: the first of its categories that is
    /// a main one, as its maker put them, so `Game;Education;` is a game
    /// and `Education;Game;` is for learning with.
    pub fn section(&self) -> &'static str {
        self.categories.iter().find_map(|c| SECTIONS.iter().find(|(_, kinds)| kinds.contains(&c.as_str())).map(|(name, _)| *name)).unwrap_or(OTHER)
    }
}

/// Programs put in their sections: each section that has any, in order of
/// name with the other programs last, and its programs in the order given.
pub fn sections(programs: &[Program]) -> Vec<(&'static str, Vec<&Program>)> {
    let mut sections: Vec<(&'static str, Vec<&Program>)> = SECTIONS.iter().map(|(name, _)| *name).chain([OTHER]).map(|name| (name, Vec::new())).collect();
    for program in programs {
        let section = program.section();
        if let Some((_, within)) = sections.iter_mut().find(|(name, _)| *name == section) {
            within.push(program);
        }
    }
    sections.retain(|(_, within)| !within.is_empty());
    sections
}

/// Read one desktop entry. None if it is not a program to be shown here:
/// not a program at all, hidden by its maker or by the user, wanting a
/// terminal to run in (there is none to give it), or meant only for some
/// other desktop.
pub fn read(text: &str) -> Option<Program> {
    read_for(text, false)
}

/// Read one desktop entry, as something to show (as `read` does) or, with
/// `startup`, as something to start when the desktop does. Those are often
/// marked as not for showing, which is no reason not to start them; what
/// rules one out is being switched off by the user.
pub fn read_for(text: &str, startup: bool) -> Option<Program> {
    let mut main = false;
    let (mut name, mut exec, mut kind, mut categories) = (None, None, None, Vec::new());
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            main = line == "[Desktop Entry]";
            continue;
        }
        let Some((key, value)) = line.split_once('=').filter(|_| main && !line.starts_with('#')) else { continue };
        let (key, value) = (key.trim(), value.trim());
        let listed = |list: &str| list.split(';').any(|d| d.eq_ignore_ascii_case("press"));
        match key {
            "Name" => name = Some(value.to_string()),
            "Exec" => exec = Some(value.to_string()),
            "Type" => kind = Some(value.to_string()),
            "Categories" => categories = value.split(';').map(str::trim).filter(|c| !c.is_empty()).map(str::to_string).collect(),
            "NoDisplay" if value == "true" && !startup => return None,
            "Hidden" | "Terminal" if value == "true" => return None,
            "X-GNOME-Autostart-enabled" if value == "false" && startup => return None,
            "OnlyShowIn" if !listed(value) => return None,
            "NotShowIn" if listed(value) => return None,
            _ => {}
        }
    }
    let command = command(&exec?);
    (kind.as_deref() == Some("Application") && !command.is_empty()).then_some(Program { name: name.filter(|n| !n.is_empty())?, command, categories })
}

/// Take a desktop entry's command line apart: words separated by spaces,
/// a word in double quotes kept whole, a backslash keeping the character
/// after it. The places where a file or an address would be put in (%f,
/// %U and the like) are left out, since none is being opened.
pub fn command(exec: &str) -> Vec<String> {
    let (mut words, mut word, mut quoted, mut any) = (Vec::new(), String::new(), false, false);
    let mut chars = exec.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => (quoted, any) = (!quoted, true),
            '\\' => word.extend(chars.next()),
            // two make a per cent sign; one with a letter is a place for
            // something to be put in, and nothing is
            '%' => {
                if let Some('%') = chars.next() {
                    word.push('%');
                }
            }
            ' ' | '\t' if !quoted => {
                if any || !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
                any = false;
            }
            ch => word.push(ch),
        }
    }
    if any || !word.is_empty() {
        words.push(word);
    }
    words
}

/// Where desktop entries are kept, the user's own first: what is found
/// there stands in for an entry of the same name further on.
pub fn folders() -> Vec<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let home = var("HOME").map(PathBuf::from).unwrap_or_default();
    let own = var("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home.join(".local/share"));
    let shared = var("XDG_DATA_DIRS").map_or_else(|| "/usr/local/share:/usr/share".into(), |v| v.to_string_lossy().into_owned());
    std::iter::once(own).chain(shared.split(':').filter(|d| d.starts_with('/')).map(PathBuf::from)).map(|d| d.join("applications")).collect()
}

/// Where the entries for programs to start with the desktop are kept:
/// `autostart` in the user's settings folder, then the system's.
pub fn startup_folders() -> Vec<PathBuf> {
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let home = var("HOME").map(PathBuf::from).unwrap_or_default();
    let own = var("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).unwrap_or_else(|| home.join(".config"));
    let shared = var("XDG_CONFIG_DIRS").map_or_else(|| "/etc/xdg".into(), |v| v.to_string_lossy().into_owned());
    std::iter::once(own).chain(shared.split(':').filter(|d| d.starts_with('/')).map(PathBuf::from)).map(|d| d.join("autostart")).collect()
}

/// Whether a program can be found to run: by its path, or along PATH.
fn runnable(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(program).is_file()))
}

/// Every program installed that can be shown and run, by name.
pub fn installed(folders: &[PathBuf]) -> Vec<Program> {
    found(folders, false)
}

/// Every program to be started with the desktop.
pub fn startup(folders: &[PathBuf]) -> Vec<Program> {
    found(folders, true)
}

fn found(folders: &[PathBuf], startup: bool) -> Vec<Program> {
    fn entries(folder: &Path, under: &str, found: &mut Vec<(String, PathBuf)>) {
        let Ok(list) = std::fs::read_dir(folder) else { return };
        for entry in list.flatten() {
            let (path, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
            if path.is_dir() {
                // an entry in a folder within is known by the folder's name and its own
                entries(&path, &format!("{under}{name}-"), found);
            } else if name.ends_with(".desktop") {
                found.push((format!("{under}{name}"), path));
            }
        }
    }
    let (mut seen, mut programs) = (HashSet::new(), Vec::new());
    for folder in folders {
        let mut found = Vec::new();
        entries(folder, "", &mut found);
        for (id, path) in found {
            // the first of a name is the one that counts, even if it says "hidden"
            if !seen.insert(id) {
                continue;
            }
            let Some(program) = std::fs::read_to_string(&path).ok().and_then(|text| read_for(&text, startup)) else { continue };
            if runnable(&program.command[0]) {
                programs.push(program);
            }
        }
    }
    programs.sort_by_key(|p| p.name.to_lowercase());
    programs.dedup_by(|a, b| a.name == b.name && a.command == b.command);
    programs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_line_is_taken_apart() {
        assert_eq!(command("gimp %U"), ["gimp"]);
        assert_eq!(command("env LANG=C \"/opt/My App/run\" --new-window %f"), ["env", "LANG=C", "/opt/My App/run", "--new-window"]);
        assert_eq!(command("printf 100%% \\\"quoted\\\" \"\""), ["printf", "100%", "\"quoted\"", ""]);
        assert!(command("  ").is_empty());
    }

    #[test]
    fn an_entry_is_read_or_passed_over() {
        let entry = |more: &str| format!("[Desktop Entry]\nType=Application\nName=Paint\nName[fr]=Peinture\nExec=paint %F\n{more}\n[Desktop Action New]\nName=New Picture\nExec=paint --new\n");
        assert_eq!(read(&entry("")), Some(Program { name: "Paint".into(), command: vec!["paint".into()], categories: vec![] }));
        for hidden in ["NoDisplay=true", "Hidden=true", "Terminal=true", "OnlyShowIn=KDE;GNOME;", "NotShowIn=press;"] {
            assert_eq!(read(&entry(hidden)), None, "{hidden}");
        }
        assert!(read(&entry("OnlyShowIn=KDE;press;")).is_some());
        assert_eq!(read("[Desktop Entry]\nType=Link\nName=Web\nURL=http://example.org\n"), None);
        assert_eq!(read("[Desktop Entry]\nType=Application\nExec=nameless\n"), None);
    }

    #[test]
    fn programs_are_put_in_sections_by_their_main_category() {
        let program = |name: &str, categories: &str| read(&format!("[Desktop Entry]\nType=Application\nName={name}\nExec={name}\nCategories={categories}\n")).unwrap();
        let programs = [
            program("Chess", "Game;BoardGame;"),
            program("Draw", "Graphics;VectorGraphics;"),
            program("Fractions", "Education;Game;Math;"),
            program("Mystery", ""),
            program("Mixer", "Audio;AudioVideo;Mixer"),
            program("Odd", "X-Own;Qt;"),
            program("Solitaire", "Game;CardGame;"),
            program("Terminal", "System;TerminalEmulator;"),
        ];
        assert_eq!(programs[4].categories, ["Audio", "AudioVideo", "Mixer"]);
        assert_eq!(programs[2].section(), "Education");
        let names: Vec<(&str, Vec<&str>)> = sections(&programs).into_iter().map(|(s, p)| (s, p.into_iter().map(|p| p.name.as_str()).collect())).collect();
        assert_eq!(
            names,
            [
                ("Education", vec!["Fractions"]),
                ("Games", vec!["Chess", "Solitaire"]),
                ("Graphics", vec!["Draw"]),
                ("Sound & Video", vec!["Mixer"]),
                ("System", vec!["Terminal"]),
                ("Other", vec!["Mystery", "Odd"]),
            ]
        );
    }

    #[test]
    fn programs_to_start_with_the_desktop_are_read_differently() {
        let entry = |more: &str| format!("[Desktop Entry]\nType=Application\nName=Helper\nExec=helper --quiet\n{more}\n");
        // not for showing, yet to be started
        assert_eq!(read(&entry("NoDisplay=true")), None);
        assert!(read_for(&entry("NoDisplay=true"), true).is_some());
        // switched off by the user, either way of saying so; or meant for another desktop
        for off in ["Hidden=true", "X-GNOME-Autostart-enabled=false", "OnlyShowIn=GNOME;", "NotShowIn=press;"] {
            assert_eq!(read_for(&entry(off), true), None, "{off}");
        }
        assert!(read(&entry("X-GNOME-Autostart-enabled=false")).is_some());
    }

    #[test]
    fn installed_programs_are_found_the_users_own_first() {
        let dir = std::env::temp_dir().join(format!("press-programs-{}", std::process::id()));
        let (own, shared) = (dir.join("own/applications"), dir.join("shared/applications"));
        std::fs::create_dir_all(own.join("kde")).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        let entry = |name: &str, exec: &str, more: &str| format!("[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\n{more}\n");
        std::fs::write(shared.join("zebra.desktop"), entry("Zebra", "/bin/sh -c true", "")).unwrap();
        std::fs::write(shared.join("apple.desktop"), entry("apple", "/bin/sh", "")).unwrap();
        // the user's own entry of the same name stands in for the shared one, here to hide it
        std::fs::write(shared.join("secret.desktop"), entry("Secret", "/bin/sh", "")).unwrap();
        std::fs::write(own.join("secret.desktop"), entry("Secret", "/bin/sh", "Hidden=true")).unwrap();
        std::fs::write(own.join("kde/tool.desktop"), entry("Mango", "/bin/sh", "")).unwrap();
        // a program that is not there to run is not offered
        std::fs::write(shared.join("gone.desktop"), entry("Gone", "/no/such/program", "")).unwrap();
        let names: Vec<String> = installed(&[own, shared]).into_iter().map(|p| p.name).collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(names, ["apple", "Mango", "Zebra"]);
    }
}
