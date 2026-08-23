use crate::task::TaskList;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const DEFAULT_FILE: &str = "tasks.json";

pub struct Storage;

impl Storage {
    pub fn save(tasks: &TaskList, path: Option<&str>) -> io::Result<()> {
        let file_path = path.unwrap_or(DEFAULT_FILE);
        let json = serde_json::to_string_pretty(tasks)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut file = fs::File::create(file_path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load(path: Option<&str>) -> io::Result<TaskList> {
        let file_path = path.unwrap_or(DEFAULT_FILE);
        if !Path::new(file_path).exists() {
            return Ok(TaskList::new());
        }
        let json = fs::read_to_string(file_path)?;
        let tasks = serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(tasks)
    }
}
