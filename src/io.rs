pub trait ErrWriter {
    fn write(&mut self, s: &str);
}

pub struct StderrWriter;

impl ErrWriter for StderrWriter {
    fn write(&mut self, s: &str) {
        eprintln!("{}", s);
    }
}

#[cfg(test)]
pub mod tests {
    use super::ErrWriter;

    #[derive(Debug, Default)]
    pub struct TestWriter {
        buffer: Vec<u8>,
    }

    impl TestWriter {
        pub fn new() -> Self {
            TestWriter { buffer: Vec::new() }
        }

        pub fn output(&self) -> String {
            String::from_utf8(self.buffer.clone()).unwrap()
        }
    }

    impl ErrWriter for TestWriter {
        fn write(&mut self, s: &str) {
            self.buffer.extend_from_slice(s.as_bytes());
        }
    }
}
