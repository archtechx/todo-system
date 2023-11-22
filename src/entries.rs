use std::path::PathBuf;

#[derive(Debug, PartialEq, Clone)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Entry {
    pub text: String,
    pub location: Location,
    pub data: EntryData,
}

#[derive(Debug, PartialEq, Clone)]
pub enum EntryData {
    Priority(isize),
    Category(String),
    Generic,
}
