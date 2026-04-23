use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EntryKind {
    App = 0,
    File = 1,
    Folder = 2,
    System = 3,
    Web = 4,
    Math = 5,
    Workflow = 6,
    Skill = 7,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub name_lower: String,
    pub path: String,
    pub subtitle: String,
    pub kind: EntryKind,
    pub score: u32,
    pub accessories: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub extra_paths: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            extra_paths: vec![],
        }
    }
}
