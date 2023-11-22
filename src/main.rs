use std::path::PathBuf;

use clap::Parser;
use crate::entries::Entry;
use crate::render::render_entries;
use crate::scan::{Stats, scan_dir, scan_todo_file};

pub mod scan;
pub mod render;
pub mod entries;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to your README.md file
    #[arg(short, long, default_value = "README.md")]
    readme: String,

    // Path to your todo.md file
    #[arg(short, long, default_value = "todo.md")]
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

    // todo@real logic for readme.md

    let mut entries: Vec<Entry> = vec![];
    let mut stats = Stats::new();

    for p in &paths {
        scan_dir(p.as_path(), &mut entries, &excludes, &mut stats).unwrap();
    }

    let mut todos_path = root_dir.clone();
    todos_path.push(&args.todos);

    if todos_path.exists() {
        scan_todo_file(&todos_path, &mut entries).unwrap();
    }

    render_entries(entries);

    if args.verbose {
        eprint!("\n\n");
        stats.print();
        eprintln!("Paths: {:?}", &paths);
        eprintln!("Excludes: {:?}", &excludes);
    }
}
