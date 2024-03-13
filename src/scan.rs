use std::io;
use std::fs::{self, canonicalize};
use std::path::{Path, PathBuf};
use glob::glob;

const PRIORITY_CHARS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

use crate::entries::{Entry, EntryData, Location};

pub struct Stats {
    visited_folder_count: usize,
    visited_file_count: usize,
    visited_folders: Vec<String>,
    visited_files: Vec<String>,
    verbosity: u8,
}

impl Stats {
    pub fn new(verbosity: u8) -> Stats {
        Stats {
            visited_folder_count: 0,
            visited_file_count: 0,
            visited_folders: vec![],
            visited_files: vec![],
            verbosity,
        }
    }

    pub fn add_file(&mut self, file: String) {
        self.visited_file_count += 1;

        if self.verbosity >= 3 {
            eprintln!("[INFO] Visited file: {}", &file);
        }

        if self.verbosity >= 2 {
            self.visited_files.push(file);
        }
    }

    pub fn add_folder(&mut self, folder: String) {
        self.visited_folder_count += 1;

        if self.verbosity >= 3 {
            eprintln!("[INFO] Visited folder: {}", &folder);
        }

        if self.verbosity >= 2 {
            self.visited_folders.push(folder);
        }
    }

    pub fn print(&self) {
        if self.verbosity >= 2 {
            eprintln!("[INFO] Visited folders:");

            for folder in &self.visited_folders {
                println!("{}", folder);
            }

            eprint!("\n\n");

            eprintln!("[INFO] Visited files:");

            for file in &self.visited_files {
                println!("{}", file);
            }

            eprint!("\n\n");
        }

        eprintln!("[INFO] Visited folders: {}", self.visited_folder_count);
        eprintln!("[INFO] Visited files: {}", self.visited_file_count);
    }
}

fn parse_priority(word: &str) -> Option<isize> {
    let lowercase_word = word.to_lowercase();
    let priority_substr = lowercase_word.split("todo").nth(1).unwrap();

    if priority_substr.len() == 1 {
        Some(priority_substr.to_string().parse::<isize>().unwrap())
    } else if priority_substr.chars().all(|ch| ch == '0') {
        // todo0: 1 - 1 = 0
        // todo00: 1 - 2 = -1
        Some(1 - priority_substr.len() as isize)
    } else {
        None // invalid syntax like todo11
    }
}

/// Remove closing tags, comments, and whitespace
fn clean_line<'a>(line: &'a str, delimiter_word: &str) -> &'a str {
    line.split_once(delimiter_word).unwrap().1
        .trim()
        .trim_end_matches("*/")
        .trim_end_matches("-->")
        .trim_end_matches("--}}")
        .trim_end_matches("/>")
        .trim()
}

pub fn add_excludes_from_gitignore(base_dir: &PathBuf, excludes: &mut Vec<PathBuf>) {
    let mut gitignore = base_dir.clone();
    gitignore.push(".gitignore");

    if ! gitignore.exists() {
        return;
    }

    for line in std::fs::read_to_string(gitignore).unwrap().lines() {
        if line.trim().is_empty() {
            continue;
        }

        if line.trim() == "*" {
            if let Ok(realpath) = canonicalize(base_dir) {
                excludes.push(realpath);
            }

            break;
        }

        if line.trim().starts_with('!') {
            continue;
        }

        if line.trim().starts_with('#') {
            continue;
        }

        let mut pattern = base_dir.clone();
        pattern.push(line.trim_end_matches("*/").trim_matches('/'));

        if let Some(pattern_str) = pattern.to_str() {
            for path in glob(pattern_str).unwrap() {
                if let Ok(exclude) = canonicalize(path.unwrap()) {
                    excludes.push(exclude);
                }
            }
        }
    }
}

pub fn scan_string(str: String, filename: PathBuf, entries: &mut Vec<Entry>) {
    for (line_num, line) in str.lines().enumerate() {
        if ! line.to_lowercase().contains("todo") {
            continue;
        }

        for mut word in line.split_whitespace() {
            if ! word.to_lowercase().starts_with("todo") {
                continue;
            }

            let text = clean_line(line, word);

            if word.starts_with("todo!(") {
                entries.push(Entry {
                    text: line.trim().to_string(),
                    location: Location {
                        file: filename.clone(),
                        line: line_num + 1,
                    },
                    data: EntryData::Generic,
                });

                break;
            }

            word = word.trim_end_matches(':');

            // Handles: `todo`, `TODO`, `todo:`, `TODO:`
            // Also trims `"` and `'` to handle cases like `foo="bar todo"`
            if word.to_lowercase().trim_end_matches('"').trim_end_matches('\'') == "todo" {
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

            if word.chars().any(|ch| PRIORITY_CHARS.contains(&ch)) {
                if let Some(priority) = parse_priority(word) {
                    entries.push(Entry {
                        text: text.to_string(),
                        location: Location {
                            file: filename.clone(),
                            line: line_num + 1,
                        },
                        data: EntryData::Priority(priority),
                    });
                }

                break;
            }
        }
    }
}

pub fn scan_file(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    if let Ok(str) = std::fs::read_to_string(path) {
        scan_string(str, path.to_path_buf(), entries);
    }

    Ok(())
}

pub fn scan_dir(dir: &Path, entries: &mut Vec<Entry>, excludes: &mut Vec<PathBuf>, stats: &mut Stats) -> io::Result<()> {
    let mut gitignore = dir.to_path_buf().clone();
    gitignore.push(".gitignore");

    if gitignore.exists() {
        add_excludes_from_gitignore(&dir.to_path_buf(), excludes);

        // `add_excludes_from_gitignore` can add the *entire* directory being scanned here to excludes
        // e.g. if it contains a `*` line. Tthe directory is visited first, and gitignore is read second,
        // so the exclude would not affect anything inside the for loop. For that reason, we re-check if
        // `dir` hasn't become excluded after running `add_excludes_from_gitignore`.
        for exclude in &*excludes {
            if canonicalize(dir).unwrap() == *exclude {
                return Ok(());
            }
        }
    }

    stats.add_folder(dir.to_string_lossy().to_string());

    'entry: for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.components().last().unwrap().as_os_str().to_string_lossy().starts_with('.') {
            continue;
        }

        for exclude in &*excludes {
            if canonicalize(&path).unwrap() == *exclude {
                continue 'entry;
            }
        }

        if path.is_dir() {
            scan_dir(path.as_path(), entries, excludes, stats)?
        } else {
            stats.add_file(path.to_string_lossy().to_string());
            scan_file(path.as_path(), entries)?
        }
    }

    Ok(())
}

pub fn scan_todo_file(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    let str = fs::read_to_string(path)?;
    let mut current_category: Option<&str> = None;

    // This can produce:
    // - generic todos (above any category)
    // - category todos (below a ## category heading)
    // - priority todos (priority keyword part of the line)
    'line: for (line_num, line) in str.lines().enumerate() {
        if line.starts_with('#') {
            current_category = Some(line.split_once("# ").unwrap().1);

            continue;
        }

        if ! line.trim_start().starts_with('-') {
            continue;
        }

        for word in line.split_whitespace() {
            if word.to_lowercase().trim_end_matches(':').starts_with("todo") && word.chars().any(|ch| PRIORITY_CHARS.contains(&ch)) {
                if let Some(priority) = parse_priority(word.trim_end_matches(':')) {
                    entries.push(Entry {
                        text: clean_line(line, word).to_string(),
                        location: Location {
                            file: path.to_path_buf(),
                            line: line_num + 1,
                        },
                        data: EntryData::Priority(priority),
                    });
                }

                continue 'line;
            }
        }

        let text = line.trim_start().trim_start_matches("- [ ] ").trim_start_matches("- ").to_string();

        if let Some(category) = current_category {
            entries.push(Entry {
                text,
                location: Location {
                    file: path.to_path_buf(),
                    line: line_num + 1,
                },
                data: EntryData::Category(category.to_string()),
            });

            continue;
        }

        entries.push(Entry {
            text,
            location: Location {
                file: path.to_path_buf(),
                line: line_num + 1,
            },
            data: EntryData::Generic,
        });
    }

    Ok(())
}

pub fn scan_readme_file(path: &Path, entries: &mut Vec<Entry>) -> io::Result<()> {
    let str = fs::read_to_string(path)?;
    let mut in_todo_section = false;

    // This can produce:
    // - generic todos (above any category)
    // - category todos (below a ## category heading) todo@real add this logic and update README.md
    // - priority todos (priority keyword part of the line)
    'line: for (line_num, line) in str.lines().enumerate() {
        if line.starts_with('#') {
            let section = line.split_once("# ").unwrap().1;
            let cleaned_section = section.to_lowercase().trim_end_matches(':').trim().to_string();

            in_todo_section = cleaned_section == "todo" || cleaned_section == "todos";

            continue;
        }

        if ! in_todo_section {
            continue;
        }

        if ! line.trim_start().starts_with('-') {
            continue;
        }

        for word in line.split_whitespace() {
            if word.to_lowercase().trim_end_matches(':').starts_with("todo") && word.chars().any(|ch| PRIORITY_CHARS.contains(&ch)) {
                if let Some(priority) = parse_priority(word.trim_end_matches(':')) {
                    entries.push(Entry {
                        text: clean_line(line, word).to_string(),
                        location: Location {
                            file: path.to_path_buf(),
                            line: line_num + 1,
                        },
                        data: EntryData::Priority(priority),
                    });
                }

                continue 'line;
            }
        }

        // README.md can only have priority entries and generic entries
        entries.push(Entry {
            text: line.trim_start().trim_start_matches("- [ ] ").trim_start_matches("- ").to_string(),
            location: Location {
                file: path.to_path_buf(),
                line: line_num + 1,
            },
            data: EntryData::Generic,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            <!-- TODO foo2 -->
            */
        "#;

        let mut entries: Vec<Entry> = vec![];
        let mut path = PathBuf::new();
        path.push("foo.txt");

        scan_string(str.to_string(), path.clone(), &mut entries);

        assert_eq!(6, entries.len());

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

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("foo2"),
            location: Location {
                file: path.clone(),
                line: 11,
            }
        }, entries[5]);
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
            <!-- TODO@baz3 -->
        "#;

        let mut entries: Vec<Entry> = vec![];
        let mut path = PathBuf::new();
        path.push("foo.txt");

        scan_string(str.to_string(), path.clone(), &mut entries);

        assert_eq!(7, entries.len());

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

        assert_eq!(Entry {
            data: EntryData::Category(String::from("baz3")),
            text: String::from(""),
            location: Location {
                file: path.clone(),
                line: 12,
            }
        }, entries[6]);
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
            <!-- TODO4 b -->
        "#;

        let mut entries: Vec<Entry> = vec![];
        let mut path = PathBuf::new();
        path.push("foo.txt");

        scan_string(str.to_string(), path.clone(), &mut entries);

        assert_eq!(10, entries.len());

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

        assert_eq!(Entry {
            data: EntryData::Priority(4),
            text: String::from("b"),
            location: Location {
                file: path.clone(),
                line: 15,
            }
        }, entries[9]);
    }

    #[test]
    fn sample_test_ts() {
        let mut entries: Vec<Entry> = vec![];

        let mut path = std::env::current_dir().unwrap();
        path.push("samples");
        path.push("1.ts");

        scan_file(path.as_path(), &mut entries).unwrap();

        assert_eq!(10, entries.len());

        assert_eq!(Entry {
            data: EntryData::Category(String::from("types")),
            text: String::from(""),
            location: Location {
                file: path.clone(),
                line: 1,
            }
        }, entries[0]);

        assert_eq!(Entry {
            data: EntryData::Category(String::from("types")),
            text: String::from("add types"),
            location: Location {
                file: path.clone(),
                line: 5,
            }
        }, entries[1]);

        assert_eq!(Entry {
            data: EntryData::Priority(-2),
            text: String::from(""),
            location: Location {
                file: path.clone(),
                line: 10,
            }
        }, entries[2]);

        assert_eq!(Entry {
            data: EntryData::Priority(-1),
            text: String::from("add return typehint"),
            location: Location {
                file: path.clone(),
                line: 14,
            }
        }, entries[3]);

        assert_eq!(Entry {
            data: EntryData::Priority(0),
            text: String::from("add name typehint"),
            location: Location {
                file: path.clone(),
                line: 19,
            }
        }, entries[4]);

        assert_eq!(Entry {
            data: EntryData::Priority(1),
            text: String::from("add return typehint"),
            location: Location {
                file: path.clone(),
                line: 23,
            }
        }, entries[5]);

        assert_eq!(Entry {
            data: EntryData::Priority(2),
            text: String::from("add return typehint"),
            location: Location {
                file: path.clone(),
                line: 27,
            }
        }, entries[6]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from(""),
            location: Location {
                file: path.clone(),
                line: 31,
            }
        }, entries[7]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("generic todo 2"),
            location: Location {
                file: path.clone(),
                line: 33,
            }
        }, entries[8]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("generic todo 3"),
            location: Location {
                file: path.clone(),
                line: 34,
            }
        }, entries[9]);
    }

    #[test]
    fn sample_test_rs() {
        let mut entries: Vec<Entry> = vec![];

        let mut path = std::env::current_dir().unwrap();
        path.push("samples");
        path.push("2.rs");

        scan_file(path.as_path(), &mut entries).unwrap();

        assert_eq!(4, entries.len());

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("todo!(\"generic\");"),
            location: Location {
                file: path.clone(),
                line: 3,
            }
        }, entries[0]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("todo!();"),
            location: Location {
                file: path.clone(),
                line: 4,
            }
        }, entries[1]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("todo!(\"@foo not category\");"),
            location: Location {
                file: path.clone(),
                line: 5,
            }
        }, entries[2]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("todo!(\"00 not priority\");"),
            location: Location {
                file: path.clone(),
                line: 6,
            }
        }, entries[3]);
    }

    #[test]
    fn todo_file_test() {
        let mut entries: Vec<Entry> = vec![];

        let mut path = std::env::current_dir().unwrap();
        path.push("samples");
        path.push("todo.md");

        scan_todo_file(path.as_path(), &mut entries).unwrap();

        assert_eq!(8, entries.len());

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("generic foo"),
            location: Location {
                file: path.clone(),
                line: 1,
            }
        }, entries[0]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("generic bar"),
            location: Location {
                file: path.clone(),
                line: 2,
            }
        }, entries[1]);

        assert_eq!(Entry {
            data: EntryData::Priority(-1),
            text: String::from("priority bar"),
            location: Location {
                file: path.clone(),
                line: 3,
            }
        }, entries[2]);

        assert_eq!(Entry {
            data: EntryData::Priority(0),
            text: String::from("a"),
            location: Location {
                file: path.clone(),
                line: 6,
            }
        }, entries[3]);

        assert_eq!(Entry {
            data: EntryData::Category(String::from("High priority")),
            text: String::from("foo"),
            location: Location {
                file: path.clone(),
                line: 7,
            }
        }, entries[4]);

        assert_eq!(Entry {
            data: EntryData::Category(String::from("High priority")),
            text: String::from("bar"),
            location: Location {
                file: path.clone(),
                line: 8,
            }
        }, entries[5]);

        assert_eq!(Entry {
            data: EntryData::Category(String::from("Responsivity")),
            text: String::from("abc"),
            location: Location {
                file: path.clone(),
                line: 11,
            }
        }, entries[6]);

        assert_eq!(Entry {
            data: EntryData::Category(String::from("Responsivity")),
            text: String::from("def"),
            location: Location {
                file: path.clone(),
                line: 12,
            }
        }, entries[7]);
    }

    #[test]
    fn readme_file_test() {
        let mut entries: Vec<Entry> = vec![];

        let mut path = std::env::current_dir().unwrap();
        path.push("samples");
        path.push("README.md");

        scan_readme_file(path.as_path(), &mut entries).unwrap();

        assert_eq!(5, entries.len());

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("abc"),
            location: Location {
                file: path.clone(),
                line: 19,
            }
        }, entries[0]);

        assert_eq!(Entry {
            data: EntryData::Priority(0),
            text: String::from("def"),
            location: Location {
                file: path.clone(),
                line: 20,
            }
        }, entries[1]);

        assert_eq!(Entry {
            data: EntryData::Priority(-1),
            text: String::from("ghi"),
            location: Location {
                file: path.clone(),
                line: 21,
            }
        }, entries[2]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("bar"),
            location: Location {
                file: path.clone(),
                line: 22,
            }
        }, entries[3]);

        assert_eq!(Entry {
            data: EntryData::Generic,
            text: String::from("baz"),
            location: Location {
                file: path.clone(),
                line: 23,
            }
        }, entries[4]);
    }
}
