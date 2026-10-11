use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS samples_a (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value REAL NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS samples_b (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                value REAL NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_value_a(&self, value: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO samples_a (value) VALUES (?1)", [value])?;
        Ok(())
    }

    pub fn insert_value_b(&self, value: f64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO samples_b (value) VALUES (?1)", [value])?;
        Ok(())
    }

    pub fn get_all_a(&self) -> Result<Vec<f64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM samples_a ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut values = Vec::new();
        for v in rows { values.push(v?); }
        Ok(values)
    }

    pub fn get_all_b(&self) -> Result<Vec<f64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM samples_b ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut values = Vec::new();
        for v in rows { values.push(v?); }
        Ok(values)
    }

    pub fn clear_a(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM samples_a", [])?;
        Ok(())
    }

    pub fn clear_b(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM samples_b", [])?;
        Ok(())
    }

    pub fn delete_last_a(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM samples_a WHERE id = (SELECT MAX(id) FROM samples_a)", [])?;
        Ok(())
    }

    pub fn delete_last_b(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM samples_b WHERE id = (SELECT MAX(id) FROM samples_b)", [])?;
        Ok(())
    }
}
