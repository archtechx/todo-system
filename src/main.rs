use std::collections::HashMap;
use std::{io, fs};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Clone)]
struct Location {
    file: PathBuf,
    line: usize,
}

#[derive(Debug, PartialEq, Clone)]
struct Entry {
    text: String,
    location: Location,
    data: EntryData,
}

#[derive(Debug, PartialEq, Clone)]
enum EntryData {
    Priority(isize),
    Category(String),
    Generic,
}

impl Entry {
    fn render(&self) {
        if self.text.len() > 0 {
            println!("- [ ] {} ({}:{})", self.text, self.location.file.to_string_lossy(), self.location.line);
        } else {
            println!("- [ ] {}:{}", self.location.file.to_string_lossy(), self.location.line);
        }
    }
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

// todo test this using sample.ts
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

fn render(entries: Vec<Entry>) {
    let mut priority_entries: HashMap<isize, Vec<Entry>> = HashMap::new();
    let mut category_entries: HashMap<String, Vec<Entry>> = HashMap::new();
    let mut generic_entries: Vec<Entry> = Vec::new();

    for entry in entries {
        match entry.data {
            EntryData::Priority(priority) => {
                if ! priority_entries.contains_key(&priority) {
                    priority_entries.insert(priority, vec![]);
                }

                let vec = priority_entries.get_mut(&priority).unwrap();
                vec.push(entry);
            },
            EntryData::Category(ref category) => {
                if ! category_entries.contains_key(category) {
                    category_entries.insert(category.clone(), vec![]);
                }

                let vec = category_entries.get_mut(category).unwrap();
                vec.push(entry);
            },
            EntryData::Generic => {
                generic_entries.push(entry);
            }
        }
    }

    print!("# TODOs\n\n");

    let mut priority_keys = priority_entries.keys().collect::<Vec<&isize>>();
    priority_keys.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for priority in priority_keys {
        let priority_notation = match priority.cmp(&0) {
            std::cmp::Ordering::Less => {
                let mut str = "todo0".to_string();

                // todo0 -> 0
                // todo00 -> -1
                // Therefore: 'todo0' + priority.abs() * '0'
                str.push_str(String::from_utf8(vec![b'0'; priority.abs() as usize]).unwrap().as_str());

                str
            },
            std::cmp::Ordering::Equal => "todo0".to_string(),
            std::cmp::Ordering::Greater => format!("todo{}", priority),
        };

        println!("## {}", priority_notation);

        for item in priority_entries.get(priority).unwrap() {
            item.render();
        }

        println!("");
    }

    let mut category_keys = category_entries.keys().collect::<Vec<&String>>();
    category_keys.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for category in category_keys {
        println!("## {}", category);

        for item in category_entries.get(category).unwrap() {
            item.render();
        }

        println!("");
    }

    println!("## Other");

    generic_entries.sort_by(|a, b| a.text.partial_cmp(&b.text).unwrap());

    for item in generic_entries {
        item.render();
    }

}

fn main() {
    let args = std::env::args();
    let mut root_dir: PathBuf = std::env::current_dir().unwrap();

    if args.len() > 1 {
        for arg in args.skip(1) {
            root_dir.push(arg);
        }
    }

    let mut entries: Vec<Entry> = vec![];

    scan_dir(root_dir.as_path(), &mut entries).unwrap();

    render(entries);
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

    scan_dir(path.as_path(), &mut entries).unwrap();

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
