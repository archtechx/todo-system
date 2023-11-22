use std::io;
use std::fs;
use std::path::{Path, PathBuf};

use crate::entries::{Entry, EntryData, Location};

pub struct Stats {
    visited_folders: usize,
    visited_files: usize,
}

impl Stats {
    pub fn new() -> Stats {
        Stats {
            visited_folders: 0,
            visited_files: 0,
        }
    }

    pub fn print(&self) {
        eprintln!("[INFO] Visited folders: {}", self.visited_folders);
        eprintln!("[INFO] Visited files: {}", self.visited_files);
    }
}

pub fn scan_string(str: String, filename: PathBuf, entries: &mut Vec<Entry>) {
    for (line_num, line) in str.lines().enumerate() {
        if ! line.to_lowercase().contains("todo") {
            continue;
        }

        for word in line.split(" ") {
            if ! word.to_lowercase().starts_with("todo") {
                continue;
            }

            // Handles: `todo`, `TODO`, `todo:`, `TODO:`
            // todo@real `replace` isnt ideal, it should only replace *after* the todo, to avoid merging eg `to:do`
            if word.to_lowercase().replace(':', "") == "todo" {
                let text_dirty = line.split_once(word).unwrap().1.replace("*/", "");
                let text = text_dirty.trim();

                entries.push(Entry {
                    text: text.to_string(),
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    },
                    data: EntryData::Generic,
                });

                break;
            }

            if word.contains('@') {
                let category = word.split('@').nth(1).unwrap();
                let text_dirty = line.split_once(word).unwrap().1.replace("*/", "");
                let text = text_dirty.trim();

                entries.push(Entry {
                    text: text.to_string(),
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    },
                    data: EntryData::Category(category.to_string()),
                });

                break;
            }

            // [0, 1, ..., 9]
            let priority_chars: Vec<char> = (0..10).map(|int| char::from_digit(int, 10).unwrap()).collect();

            if word.chars().any(|ch| priority_chars.contains(&ch)) {
                let cleaned_word = word.to_lowercase();
                let priority_chars = cleaned_word.split("todo").nth(1).unwrap();

                let priority: isize;

                if priority_chars.len() == 1 {
                    priority = priority_chars.to_string().parse::<isize>().unwrap();
                } else if priority_chars.chars().all(|ch| ch == '0') {
                    // todo0: 1 - 1 = 0
                    // todo00: 1 - 2 = -1
                    priority = 1 - priority_chars.len() as isize;
                } else {
                    break; // incorrect syntax like todo11
                }

                let text_dirty = line.split_once(word).unwrap().1.replace("*/", "");
                let text = text_dirty.trim();

                entries.push(Entry {
                    text: text.to_string(),
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    },
                    data: EntryData::Priority(priority),
                });
            }
        }
    }
}

pub fn scan_file(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    match std::fs::read_to_string(path) {
        Ok(str) => scan_string(str, path.to_path_buf(), entries),
        Err(_) => (),
    };

    Ok(())
}

pub fn scan_dir(path: &Path, entries: &mut Vec<Entry>, excludes: &Vec<PathBuf>, stats: &mut Stats) -> io::Result<()> {
    stats.visited_folders += 1;

    'entry: for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.components().last().unwrap().as_os_str().to_string_lossy().starts_with('.') {
            continue;
        }

        if path.is_dir() {
            for exclude in excludes {
                if path == *exclude {
                    continue 'entry;
                }
            }

            scan_dir(path.as_path(), entries, excludes, stats)?
        } else {
            stats.visited_files += 1;
            scan_file(path.as_path(), entries)?
        }
    }

    Ok(())
}


#[test]
fn generic_test() {
    let str = r#"
        1
        2
        // todo foo
        /* TODO: foo bar */
        /*

        * TODO baz
        TODO baz2
        TODO baz2 todo
        */
    "#;

    let mut entries: Vec<Entry> = vec![];
    let mut path = PathBuf::new();
    path.push("foo.txt");

    scan_string(str.to_string(), path.clone(), &mut entries);

    assert_eq!(5, entries.len());

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("foo"),
        location: Location {
            file: path.clone(),
            line: 4,
        }
    }, entries[0]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("foo bar"),
        location: Location {
            file: path.clone(),
            line: 5,
        }
    }, entries[1]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("baz"),
        location: Location {
            file: path.clone(),
            line: 8,
        }
    }, entries[2]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("baz2"),
        location: Location {
            file: path.clone(),
            line: 9,
        }
    }, entries[3]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("baz2 todo"),
        location: Location {
            file: path.clone(),
            line: 10,
        }
    }, entries[4]);
}

#[test]
fn category_test() {
    let str = r#"
        1
        2
        todo@foo
        todo@bar abc def
        3
        todo@baz x y
        4
        // TODO@baz2 a
        /* TODO@baz3 */
        // TODO@baz3 b
    "#;

    let mut entries: Vec<Entry> = vec![];
    let mut path = PathBuf::new();
    path.push("foo.txt");

    scan_string(str.to_string(), path.clone(), &mut entries);

    assert_eq!(6, entries.len());

    assert_eq!(Entry {
        data: EntryData::Category(String::from("foo")),
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 4,
        }
    }, entries[0]);

    assert_eq!(Entry {
        data: EntryData::Category(String::from("bar")),
        text: String::from("abc def"),
        location: Location {
            file: path.clone(),
            line: 5,
        }
    }, entries[1]);

    assert_eq!(Entry {
        data: EntryData::Category(String::from("baz")),
        text: String::from("x y"),
        location: Location {
            file: path.clone(),
            line: 7,
        }
    }, entries[2]);

    assert_eq!(Entry {
        data: EntryData::Category(String::from("baz2")),
        text: String::from("a"),
        location: Location {
            file: path.clone(),
            line: 9,
        }
    }, entries[3]);

    assert_eq!(Entry {
        data: EntryData::Category(String::from("baz3")),
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 10,
        }
    }, entries[4]);

    assert_eq!(Entry {
        data: EntryData::Category(String::from("baz3")),
        text: String::from("b"),
        location: Location {
            file: path.clone(),
            line: 11,
        }
    }, entries[5]);
}

#[test]
fn priority_test() {
    let str = r#"
        1
        2
        todo00
        todo000 abc
        todo0 abc def
        todo1 foo
        3
        todo1 x y
        4
        // todo0 bar
        // TODO1 a
        /* TODO2 */
        // TODO3 b
    "#;

    let mut entries: Vec<Entry> = vec![];
    let mut path = PathBuf::new();
    path.push("foo.txt");

    scan_string(str.to_string(), path.clone(), &mut entries);

    assert_eq!(9, entries.len());

    assert_eq!(Entry {
        data: EntryData::Priority(-1),
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 4,
        }
    }, entries[0]);

    assert_eq!(Entry {
        data: EntryData::Priority(-2),
        text: String::from("abc"),
        location: Location {
            file: path.clone(),
            line: 5,
        }
    }, entries[1]);

    assert_eq!(Entry {
        data: EntryData::Priority(0),
        text: String::from("abc def"),
        location: Location {
            file: path.clone(),
            line: 6,
        }
    }, entries[2]);

    assert_eq!(Entry {
        data: EntryData::Priority(1),
        text: String::from("foo"),
        location: Location {
            file: path.clone(),
            line: 7,
        }
    }, entries[3]);

    assert_eq!(Entry {
        data: EntryData::Priority(1),
        text: String::from("x y"),
        location: Location {
            file: path.clone(),
            line: 9,
        }
    }, entries[4]);

    assert_eq!(Entry {
        data: EntryData::Priority(0),
        text: String::from("bar"),
        location: Location {
            file: path.clone(),
            line: 11,
        }
    }, entries[5]);

    assert_eq!(Entry {
        data: EntryData::Priority(1),
        text: String::from("a"),
        location: Location {
            file: path.clone(),
            line: 12,
        }
    }, entries[6]);

    assert_eq!(Entry {
        data: EntryData::Priority(2),
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 13,
        }
    }, entries[7]);

    assert_eq!(Entry {
        data: EntryData::Priority(3),
        text: String::from("b"),
        location: Location {
            file: path.clone(),
            line: 14,
        }
    }, entries[8]);
}

#[test]
fn sample_test_ts() {
    let mut entries: Vec<Entry> = vec![];

    let mut path = std::env::current_dir().unwrap();
    path.push("samples");

    let mut filepath = path.clone();
    filepath.push("1.ts");

    let excludes: Vec<PathBuf> = vec![];
    let mut stats = Stats::new();

    scan_dir(path.as_path(), &mut entries, &excludes, &mut stats).unwrap();

    assert_eq!(10, entries.len());

    assert_eq!(Entry {
        data: EntryData::Category(String::from("types")),
        text: String::from(""),
        location: Location {
            file: filepath.clone(),
            line: 1,
        }
    }, entries[0]);

    assert_eq!(Entry {
        data: EntryData::Category(String::from("types")),
        text: String::from("add types"),
        location: Location {
            file: filepath.clone(),
            line: 5,
        }
    }, entries[1]);

    assert_eq!(Entry {
        data: EntryData::Priority(-2),
        text: String::from(""),
        location: Location {
            file: filepath.clone(),
            line: 10,
        }
    }, entries[2]);

    assert_eq!(Entry {
        data: EntryData::Priority(-1),
        text: String::from("add return typehint"),
        location: Location {
            file: filepath.clone(),
            line: 14,
        }
    }, entries[3]);

    assert_eq!(Entry {
        data: EntryData::Priority(0),
        text: String::from("add name typehint"),
        location: Location {
            file: filepath.clone(),
            line: 19,
        }
    }, entries[4]);

    assert_eq!(Entry {
        data: EntryData::Priority(1),
        text: String::from("add return typehint"),
        location: Location {
            file: filepath.clone(),
            line: 23,
        }
    }, entries[5]);

    assert_eq!(Entry {
        data: EntryData::Priority(2),
        text: String::from("add return typehint"),
        location: Location {
            file: filepath.clone(),
            line: 27,
        }
    }, entries[6]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from(""),
        location: Location {
            file: filepath.clone(),
            line: 31,
        }
    }, entries[7]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("generic todo 2"),
        location: Location {
            file: filepath.clone(),
            line: 33,
        }
    }, entries[8]);

    assert_eq!(Entry {
        data: EntryData::Generic,
        text: String::from("generic todo 3"),
        location: Location {
            file: filepath.clone(),
            line: 34,
        }
    }, entries[9]);
}
