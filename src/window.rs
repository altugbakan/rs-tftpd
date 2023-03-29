use std::{collections::VecDeque, error::Error, fs::File, io::Read};

pub struct Window {
    elements: VecDeque<Vec<u8>>,
    size: usize,
    chunk_size: usize,
    file: File,
}

impl Window {
    pub fn new(size: usize, chunk_size: usize, file: File) -> Window {
        Window {
            elements: VecDeque::new(),
            size,
            chunk_size,
            file,
        }
    }

    pub fn fill(&mut self) -> Result<(), Box<dyn Error>> {
        for _ in self.elements.len()..self.size {
            let mut chunk = vec![0; self.chunk_size];
            let size = self.file.read(&mut chunk)?;
            self.elements.push_back(chunk);

            if size != self.chunk_size {
                break;
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, amount: usize) -> Result<(), Box<dyn Error>> {
        if amount > self.elements.len() {
            return Err("amount cannot be larger than size".into());
        }

        drop(self.elements.drain(0..amount));

        Ok(())
    }
}
