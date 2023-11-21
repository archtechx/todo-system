use std::{io, fs};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
struct Location {
    file: PathBuf,
    line: usize,
}

#[derive(Debug, PartialEq)]
struct PriorityEntry {
    text: String,
    priority: isize,
    location: Location,
}

#[derive(Debug, PartialEq)]
struct CategoryEntry {
    text: String,
    category: String,
    location: Location,
}

#[derive(Debug, PartialEq)]
struct GenericEntry {
    text: String,
    location: Location,
}

#[derive(Debug, PartialEq)]
enum Entry {
    Priority(PriorityEntry),
    Category(CategoryEntry),
    Generic(GenericEntry),
}

fn scan_string(str: String, filename: PathBuf, entries: &mut Vec<Entry>) {
    for (line_num, line) in str.lines().enumerate() {
        if ! line.to_lowercase().contains("todo") {
            continue;
        }

        for word in line.split(" ") {
            if ! word.to_lowercase().starts_with("todo") {
                continue;
            }

            // Handles: `todo`, `TODO`, `todo:`, `TODO:`
            // todo `replace` isnt ideal, it should only replace *after* the todo, to avoid merging eg `to:do`
            if word.to_lowercase().replace(':', "") == "todo" {
                let text_dirty = line.split(word).nth(1).unwrap().replace("*/", "");
                let text = text_dirty.trim();

                entries.push(Entry::Generic(GenericEntry {
                    text: text.to_string(),
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    }
                }));

                break;
            }

            if word.contains('@') {
                let category = word.split('@').nth(1).unwrap();
                let text_dirty = line.split(word).nth(1).unwrap().replace("*/", "");
                let text = text_dirty.trim();

                entries.push(Entry::Category(CategoryEntry {
                    text: text.to_string(),
                    category: category.to_string(),
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    },
                }));

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

                let text_dirty = line.split(word).nth(1).unwrap().replace("*/", "");
                let text = text_dirty.trim();

                entries.push(Entry::Priority(PriorityEntry {
                    text: text.to_string(),
                    priority: priority,
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    }
                }));
            }
        }
    }
}

fn scan_file(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    match std::fs::read_to_string(path) {
        Ok(str) => scan_string(str, path.to_path_buf(), entries),
        Err(_) => (),
    };

    Ok(())
}

fn scan_dir(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_dir(path.as_path(), entries)?
        } else {
            scan_file(path.as_path(), entries)?
        }
    }

    Ok(())
}

fn main() {
    let root_dir: PathBuf = std::env::current_dir().unwrap(); // todo@CLI make the root dir configurable

    let mut entries: Vec<Entry> = vec![];

    scan_dir(root_dir.as_path(), &mut entries).unwrap();

    dbg!(entries);
}

#[test]
fn generic_test() {
    let str = r#"
        1
        2
        // todo foo
        /* TODO: bar */
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

    assert_eq!(Entry::Generic(GenericEntry {
        text: String::from("foo"),
        location: Location {
            file: path.clone(),
            line: 4,
        }
    }), entries[0]);

    assert_eq!(Entry::Generic(GenericEntry {
        text: String::from("bar"),
        location: Location {
            file: path.clone(),
            line: 5,
        }
    }), entries[1]);

    assert_eq!(Entry::Generic(GenericEntry {
        text: String::from("baz"),
        location: Location {
            file: path.clone(),
            line: 8,
        }
    }), entries[2]);

    assert_eq!(Entry::Generic(GenericEntry {
        text: String::from("baz2"),
        location: Location {
            file: path.clone(),
            line: 9,
        }
    }), entries[3]);

    assert_eq!(Entry::Generic(GenericEntry {
        text: String::from("baz2 todo"),
        location: Location {
            file: path.clone(),
            line: 10,
        }
    }), entries[4]);
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

    assert_eq!(Entry::Category(CategoryEntry {
        category: String::from("foo"),
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 4,
        }
    }), entries[0]);

    assert_eq!(Entry::Category(CategoryEntry {
        category: String::from("bar"),
        text: String::from("abc def"),
        location: Location {
            file: path.clone(),
            line: 5,
        }
    }), entries[1]);

    assert_eq!(Entry::Category(CategoryEntry {
        category: String::from("baz"),
        text: String::from("x y"),
        location: Location {
            file: path.clone(),
            line: 7,
        }
    }), entries[2]);

    assert_eq!(Entry::Category(CategoryEntry {
        category: String::from("baz2"),
        text: String::from("a"),
        location: Location {
            file: path.clone(),
            line: 9,
        }
    }), entries[3]);

    assert_eq!(Entry::Category(CategoryEntry {
        category: String::from("baz3"),
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 10,
        }
    }), entries[4]);

    assert_eq!(Entry::Category(CategoryEntry {
        category: String::from("baz3"),
        text: String::from("b"),
        location: Location {
            file: path.clone(),
            line: 11,
        }
    }), entries[5]);
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

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: -1,
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 4,
        }
    }), entries[0]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: -2,
        text: String::from("abc"),
        location: Location {
            file: path.clone(),
            line: 5,
        }
    }), entries[1]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 0,
        text: String::from("abc def"),
        location: Location {
            file: path.clone(),
            line: 6,
        }
    }), entries[2]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 1,
        text: String::from("foo"),
        location: Location {
            file: path.clone(),
            line: 7,
        }
    }), entries[3]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 1,
        text: String::from("x y"),
        location: Location {
            file: path.clone(),
            line: 9,
        }
    }), entries[4]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 0,
        text: String::from("bar"),
        location: Location {
            file: path.clone(),
            line: 11,
        }
    }), entries[5]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 1,
        text: String::from("a"),
        location: Location {
            file: path.clone(),
            line: 12,
        }
    }), entries[6]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 2,
        text: String::from(""),
        location: Location {
            file: path.clone(),
            line: 13,
        }
    }), entries[7]);

    assert_eq!(Entry::Priority(PriorityEntry {
        priority: 3,
        text: String::from("b"),
        location: Location {
            file: path.clone(),
            line: 14,
        }
    }), entries[8]);
}
