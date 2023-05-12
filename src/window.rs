use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{Read, Write},
};

/// Window `struct` is used to store chunks of data from a file. It is
/// used to help store the data that is being sent or received for the
/// [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize option.
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

    /// Empties the `Window` by writing the data to the file.
    pub fn empty(&mut self) -> Result<(), Box<dyn Error>> {
        for data in &self.elements {
            self.file.write_all(data)?;
        }

        self.elements.clear();

        Ok(())
    }

    /// Removes the first `amount` of elements from the `Window`.
    pub fn remove(&mut self, amount: u16) -> Result<(), &'static str> {
        if amount > self.len() {
            return Err("amount cannot be larger than length of window");
        }

        drop(self.elements.drain(0..amount as usize));

        Ok(())
    }

    /// Adds a data `Vec<u8>` to the `Window`.
    pub fn add(&mut self, data: Vec<u8>) -> Result<(), &'static str> {
        if self.len() == self.size {
            return Err("cannot add to a full window");
        }

        self.elements.push_back(data);

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

    /// Returns `true` if the `Window` is full.
    pub fn is_full(&self) -> bool {
        self.elements.len() as u16 == self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, OpenOptions},
        io::{Seek, Write},
        path::Path,
    };

    const DIR_NAME: &str = "tmp";

    #[test]
    fn fills_and_removes_from_window() {
        const FILE_NAME: &str = "fills_and_removes_from_window.txt";

        let mut file = initialize(FILE_NAME);
        file.write_all(b"Hello, world!").unwrap();
        file.flush().unwrap();
        file.rewind().unwrap();

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

        clean(FILE_NAME);
    }

    #[test]
    fn adds_to_and_empties_window() {
        const FILE_NAME: &str = "adds_to_and_empties_window.txt";

        let file = initialize(FILE_NAME);

        let mut window = Window::new(3, 5, file);
        window.add(b"Hello".to_vec()).unwrap();
        assert_eq!(window.elements.len(), 1);
        assert_eq!(window.elements[0], b"Hello"[..]);

        window.add(b", wor".to_vec()).unwrap();
        assert_eq!(window.elements.len(), 2);
        assert_eq!(window.elements[0], b"Hello"[..]);
        assert_eq!(window.elements[1], b", wor"[..]);

        window.add(b"ld!".to_vec()).unwrap();
        assert_eq!(window.elements.len(), 3);
        assert_eq!(window.elements[0], b"Hello"[..]);
        assert_eq!(window.elements[1], b", wor"[..]);
        assert_eq!(window.elements[2], b"ld!"[..]);

        window.empty().unwrap();
        assert_eq!(window.elements.len(), 0);

        let mut contents = Default::default();
        File::read_to_string(
            &mut File::open(DIR_NAME.to_string() + "/" + FILE_NAME).unwrap(),
            &mut contents,
        )
        .unwrap();
        assert_eq!(contents, "Hello, world!");

        clean(FILE_NAME);
    }

    fn initialize(file_name: &str) -> File {
        let file_name = DIR_NAME.to_string() + "/" + file_name;
        if !Path::new(DIR_NAME).is_dir() {
            fs::create_dir(DIR_NAME).unwrap();
        }

        if File::open(&file_name).is_ok() {
            fs::remove_file(&file_name).unwrap();
        }

        OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&file_name)
            .unwrap()
    }

    #[allow(unused_must_use)]
    fn clean(file_name: &str) {
        let file_name = DIR_NAME.to_string() + "/" + file_name;
        fs::remove_file(file_name).unwrap();
        fs::remove_dir(DIR_NAME);
    }
}
