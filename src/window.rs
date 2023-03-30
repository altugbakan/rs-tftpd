use std::{collections::VecDeque, error::Error, fs::File, io::Read};

/// Window `struct` is used to store chunks of data from a file.
/// It is used to store the data that is being sent or received for
/// Windowsize option.
///
/// # Example
/// ```rust
/// use std::{fs::{self, OpenOptions, File}, io::Write};
/// use tftpd::Window;
///
/// let mut file = File::create("test.txt").unwrap();
/// file.write_all(b"Hello, world!").unwrap();
/// file.flush().unwrap();
///
/// let file = File::open("test.txt").unwrap();
/// let mut window = Window::new(5, 512, file);
/// window.fill().unwrap();
/// fs::remove_file("test.txt").unwrap();
/// ```
pub struct Window {
    elements: VecDeque<Vec<u8>>,
    size: u16,
    chunk_size: usize,
    file: File,
}

impl Window {
    /// Creates a new `Window` with the supplied size and chunk size.
    pub fn new(size: u16, chunk_size: usize, file: File) -> Window {
        Window {
            elements: VecDeque::new(),
            size,
            chunk_size,
            file,
        }
    }

    /// Fills the `Window` with chunks of data from the file.
    /// Returns `true` if the `Window` is full.
    pub fn fill(&mut self) -> Result<bool, Box<dyn Error>> {
        for _ in self.len()..self.size {
            let mut chunk = vec![0; self.chunk_size];
            let size = self.file.read(&mut chunk)?;

            if size != self.chunk_size {
                chunk.truncate(size);
                self.elements.push_back(chunk);
                return Ok(false);
            }

            self.elements.push_back(chunk);
        }

        Ok(true)
    }

    /// Removes the first `amount` of elements from the `Window`.
    pub fn remove(&mut self, amount: u16) -> Result<(), Box<dyn Error>> {
        if amount > self.len() {
            return Err("amount cannot be larger than size".into());
        }

        drop(self.elements.drain(0..amount as usize));

        Ok(())
    }

    /// Returns a reference to the `VecDeque` containing the elements.
    pub fn get_elements(&self) -> &VecDeque<Vec<u8>> {
        &self.elements
    }

    /// Returns the length of the `Window`.
    pub fn len(&self) -> u16 {
        self.elements.len() as u16
    }

    /// Returns `true` if the `Window` is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Write};

    #[test]
    fn fills_and_removes_from_window() {
        let file = initialize();

        let mut window = Window::new(2, 5, file);
        window.fill().unwrap();
        assert_eq!(window.elements.len(), 2);
        assert_eq!(window.elements[0], b"Hello"[..]);
        assert_eq!(window.elements[1], b", wor"[..]);

        window.remove(1).unwrap();
        assert_eq!(window.elements.len(), 1);
        assert_eq!(window.elements[0], b", wor"[..]);

        window.fill().unwrap();
        assert_eq!(window.elements.len(), 2);
        assert_eq!(window.elements[0], b", wor"[..]);
        assert_eq!(window.elements[1], b"ld!"[..]);

        clean();
    }

    fn initialize() -> File {
        let dir_name = "tmp";
        let file_name = "tmp/test.txt";

        if fs::metadata(dir_name).is_err() {
            fs::create_dir(dir_name).unwrap();
        }

        if File::open(file_name).is_ok() {
            fs::remove_file(file_name).unwrap();
        }

        let mut file = File::create(file_name).unwrap();
        file.write_all(b"Hello, world!").unwrap();
        file.flush().unwrap();

        File::open(file_name).unwrap()
    }

    fn clean() {
        fs::remove_file("tmp/test.txt").unwrap();
        fs::remove_dir("tmp").unwrap();
    }
}
