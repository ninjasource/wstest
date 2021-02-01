use byteorder::{ByteOrder, LittleEndian};

#[derive(Debug)]
pub enum FrameReaderError {
    InvalidFrameLength(usize),
}

pub struct FrameReader<'a> {
    cursor: usize,
    buffer: &'a [u8],
}

impl<'a> FrameReader<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { cursor: 0, buffer }
    }
}

impl<'a> Iterator for FrameReader<'a> {
    type Item = Result<&'a [u8], FrameReaderError>;

    fn next(&mut self) -> Option<Result<&'a [u8], FrameReaderError>> {
        let from = self.cursor + 2;
        if from < self.buffer.len() {
            // the first two bytes of the payload are the length of the payload as a u16
            let len = LittleEndian::read_u16(&self.buffer[self.cursor..]) as usize;

            // we use the payload length instead of the frame_length from the decoder
            let to = from + len;

            if to > self.buffer.len() {
                return Some(Err(FrameReaderError::InvalidFrameLength(len)));
            }

            let slice = &self.buffer[from..to];
            self.cursor = to;

            Some(Ok(slice))
        } else {
            None
        }
    }
}
