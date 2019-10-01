use std::fs::{OpenOptions, File};
use std::io::{BufWriter, Write};

pub struct Buffer {
    pub name: String,
    pub contents: Vec<String>,
    pub is_dirty: bool,
}

impl Buffer {
    pub fn save(&mut self) {
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.name)
            .unwrap();

        // let f = File::open(&self.name).unwrap();

        let mut f = BufWriter::new(f);
        for line in &self.contents {
            write!(&mut f, "{}\n", line);
            // f.write_all(line.as_bytes()).unwrap();
        }
        f.flush().unwrap();
        self.is_dirty = false;
    }

}
