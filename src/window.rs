use std::{
    collections::VecDeque,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
};

/// WindowRead `struct` is used to store chunks of data from a file. It is
/// used to help store the data that is being sent for the
/// [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize option.
///
/// # Example
/// ```rust
/// use std::{fs::{self, OpenOptions, File}, io::Write};
/// use tftpd::WindowRead;
///
/// let mut file = File::create("test.txt").unwrap();
/// file.write_all(b"Hello, world!").unwrap();
/// file.flush().unwrap();
///
/// let file = File::open("test.txt").unwrap();
/// let mut window = WindowRead::new(5, 512, file);
/// window.fill().unwrap();
/// fs::remove_file("test.txt").unwrap();
/// ```
pub struct WindowRead {
    elements: VecDeque<Vec<u8>>,
    size: u16,
    chunk_size: u16,
    bufreader: BufReader<File>,
}

impl WindowRead {
    /// Creates a new `Window` with the supplied size and chunk size.
    pub fn new(size: u16, chunk_size: u16, file: File) -> WindowRead {
        WindowRead {
            elements: VecDeque::new(),
            size,
            chunk_size,
            bufreader: BufReader::with_capacity(
                2 * size as usize*chunk_size as usize,
                file,
            ),
        }
    }

    /// Fills the `Window` with chunks of data from the file.
    /// Returns `true` if the `Window` is full.
    pub fn fill(&mut self) -> Result<bool, Box<dyn Error>> {
        for _ in self.len()..self.size {
            let mut chunk = vec![0; self.chunk_size as usize];
            let size = self.bufreader.read(&mut chunk)?;
            if size != self.chunk_size as usize {
                chunk.truncate(size);
                self.elements.push_back(chunk);
                return Ok(false);
            }

            self.elements.push_back(chunk);
        }

        Ok(true)
    }

    /// Fill the read buffer to speed up next window fill
    pub fn prefill(&mut self) -> Result<(), Box<dyn Error>> {
        self.bufreader.fill_buf()?;
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


/// WindowWrite `struct` is used to store data and write them in a file. 
/// It is used to help store the data that is being received for the
/// [RFC 7440](https://www.rfc-editor.org/rfc/rfc7440) Windowsize option.
///
/// # Example
/// ```rust
/// use std::{fs::{self, OpenOptions, File}, io::Write};
/// use tftpd::WindowWrite;
///
/// let file = File::create("test.txt").unwrap();
/// let mut window = WindowWrite::new(5, file);
/// window.add(vec![0x1, 0x2, 0x3]).unwrap();
/// window.add(vec![0x4, 0x5, 0x6]).unwrap();
/// window.empty().unwrap();
/// ```
pub struct WindowWrite {
    elements: VecDeque<Vec<u8>>,
    size: u16,
    file: File,
}

impl WindowWrite {
    /// Creates a new `Window` with the supplied size and chunk size.
    pub fn new(size: u16, file: File) -> WindowWrite {
        WindowWrite {
            elements: VecDeque::new(),
            size,
            file,
        }
    }

    /// Empties the `Window` by writing the data to the file.
    pub fn empty(&mut self) -> Result<(), Box<dyn Error>> {
        for data in &self.elements {
            self.file.write_all(data)?;
        }

        self.elements.clear();

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

    /// Returns the length of the file
    pub fn file_len(&self) -> Result<u64, Box<dyn Error>> {
        Ok(self.file.metadata()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, OpenOptions},
        io::Write,
    };

    const DIR_NAME: &str = "target/test";

    #[test]
    fn fills_and_removes_from_window() {
        const FILENAME: &str = "fills_and_removes_from_window.txt";

        let mut file = initialize(FILENAME);
        file.write_all(b"Hello, world!").unwrap();
        file.flush().unwrap();
        drop(file);

        file = open(FILENAME);

        let mut window = WindowRead::new(2, 5, file);
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

        clean(FILENAME);
    }

    #[test]
    fn adds_to_and_empties_window() {
        const FILENAME: &str = "adds_to_and_empties_window.txt";

        let file = initialize(FILENAME);

        let mut window = WindowWrite::new(3, file);
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
            &mut File::open(DIR_NAME.to_string() + "/" + FILENAME).unwrap(),
            &mut contents,
        )
        .unwrap();
        assert_eq!(contents, "Hello, world!");

        clean(FILENAME);
    }

    fn initialize(filename: &str) -> File {
        let filename = DIR_NAME.to_string() + "/" + filename;

        let _ = fs::create_dir_all(DIR_NAME);

        if File::open(&filename).is_ok() {
            fs::remove_file(&filename).unwrap();
        }

        OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&filename)
            .unwrap()
    }

    fn open(filename: &str) -> File {
        let filename = DIR_NAME.to_string() + "/" + filename;

        OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(filename)
            .unwrap()
    }

    fn clean(filename: &str) {
        let filename = DIR_NAME.to_string() + "/" + filename;
        fs::remove_file(filename).unwrap();
        if fs::remove_dir(DIR_NAME).is_err() {
            // ignore removing directory, as other tests are
            // still running
        }
    }
}
