use std::collections::HashMap;
use std::io::{self, Write};
use std::fs;
use std::path::{Path, PathBuf};
use clap::Parser;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use std::cmp::Ordering::{Less, Equal, Greater};

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
        let mut stdout = StandardStream::stdout(ColorChoice::Auto);

        write_ansi(&mut stdout, Color::Ansi256(243), "- [ ] ", false);

        let location = format!("{}:{}", self.location.file.to_string_lossy(), self.location.line);

        if self.text.len() > 0 {
            write_ansi(&mut stdout, Color::Blue, self.text.as_str(), true);

            write_ansi(&mut stdout, Color::Ansi256(243), format!(" ({})", location).as_str(), false);
        } else {
            write_ansi(&mut stdout, Color::Cyan, &location.as_str(), true);
        }

        write!(&mut stdout, "\n").unwrap();
    }
}

fn write_ansi(stdout: &mut StandardStream, color: Color, text: &str, bold: bool) {
    stdout.set_color(
    ColorSpec::new()
            .set_fg(Some(color))
            .set_bold(bold)
    ).unwrap();

    write!(stdout, "{text}").unwrap();

    stdout.reset().unwrap();
}

struct Stats {
    visited_folders: usize,
    visited_files: usize,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            visited_folders: 0,
            visited_files: 0,
        }
    }

    fn print(&self) {
        eprintln!("[INFO] Visited folders: {}", self.visited_folders);
        eprintln!("[INFO] Visited files: {}", self.visited_files);
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

fn scan_file(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    match std::fs::read_to_string(path) {
        Ok(str) => scan_string(str, path.to_path_buf(), entries),
        Err(_) => (),
    };

    Ok(())
}

fn scan_dir(path: &Path, entries: &mut Vec<Entry>, excludes: &Vec<PathBuf>, stats: &mut Stats) -> io::Result<()> {
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

fn render(entries: Vec<Entry>) {
    let mut priority_entries: HashMap<isize, Vec<Entry>> = HashMap::new();
    let mut category_entries: HashMap<String, Vec<Entry>> = HashMap::new();
    let mut generic_entries: Vec<Entry> = Vec::new();

    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

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

    write_ansi(&mut stdout, Color::Yellow, "# TODOs", true);
    write!(stdout, "\n\n").unwrap();

    let mut priority_keys = priority_entries.keys().collect::<Vec<&isize>>();
    priority_keys.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for priority in priority_keys {
        let priority_notation = match priority.cmp(&0) {
            Less => {
                let mut str = "todo0".to_string();

                // todo0 -> 0
                // todo00 -> -1
                // Therefore: 'todo0' + priority.abs() * '0'
                str.push_str(String::from_utf8(vec![b'0'; priority.abs() as usize]).unwrap().as_str());

                str
            },
            Equal => "todo0".to_string(),
            Greater => format!("todo{}", priority),
        };

        write_ansi(&mut stdout, Color::Red, format!("## {}", &priority_notation).as_str(), true);
        write!(stdout, "\n").unwrap();

        for item in priority_entries.get(priority).unwrap() {
            item.render();
        }

        println!("");
    }

    let mut category_keys = category_entries.keys().collect::<Vec<&String>>();
    category_keys.sort_by(|a, b| a.partial_cmp(b).unwrap());

    for category in category_keys {
        write_ansi(&mut stdout, Color::Green, format!("## {}", &category).as_str(), true);
        write!(stdout, "\n").unwrap();

        for item in category_entries.get(category).unwrap() {
            item.render();
        }

        println!("");
    }

    write_ansi(&mut stdout, Color::White, "## Other", true);
    write!(stdout, "\n").unwrap();

    generic_entries.sort_by(|a, b| a.text.partial_cmp(&b.text).unwrap());

    for item in generic_entries {
        item.render();
    }

}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to your README.md file
    #[arg(short, long, default_value = "")]
    readme: String,

    // Path to your todo.md file
    #[arg(short, long, default_value = "")]
    todos: String,

    // Paths to search
    #[arg(default_values_t = Vec::from([".".to_string()]))]
    paths: Vec<String>,

    // Paths to exclude
    #[arg(short, long, default_values_t = Vec::from([
        "node_modules".to_string(),
        "vendor".to_string(),
    ]))]
    exclude: Vec<String>,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    let root_dir: PathBuf = std::env::current_dir().unwrap();
    let mut paths: Vec<PathBuf> = vec![];
    let mut excludes: Vec<PathBuf> = vec![];

    for p in args.paths {
        let mut path = root_dir.clone();
        path.push(p);

        paths.push(path);
    }

    for exclude in args.exclude {
        let mut path = root_dir.clone();
        path.push(exclude);

        excludes.push(path);
    }

    // todo@real logic for readme.md and todos.md

    let mut entries: Vec<Entry> = vec![];
    let mut stats = Stats::new();

    scan_dir(root_dir.as_path(), &mut entries, &excludes, &mut stats).unwrap();

    render(entries);

    if args.verbose {
        eprint!("\n\n");
        stats.print();
    }
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
