use std::io::Write;
use std::collections::HashMap;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use std::cmp::Ordering::{Less, Equal, Greater};

use crate::entries::{Entry, EntryData};

impl Entry {
    pub fn render(&self) {
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


pub fn write_ansi(stdout: &mut StandardStream, color: Color, text: &str, bold: bool) {
    stdout.set_color(
    ColorSpec::new()
            .set_fg(Some(color))
            .set_bold(bold)
    ).unwrap();

    write!(stdout, "{text}").unwrap();

    stdout.reset().unwrap();
}

pub fn render_entries(entries: Vec<Entry>) {
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
