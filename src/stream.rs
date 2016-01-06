use std::io::prelude::*;
use std::io::Error as IoError;


pub trait BufferedTransfer {
    fn write_all_to<W: Write>(&mut self, ostream: &mut W, ) -> Result<(), IoError>;
}


impl<R: Read> BufferedTransfer for R {

    fn write_all_to<W: Write>(&mut self, ostream: &mut W) -> Result<(), IoError> {
        let mut buf = vec![0; 4096];
        let mut done = false;
        while !done {
            let n = try!(self.read(&mut buf));
            if n > 0 {
                try!(ostream.write_all(&buf[..n]))
            }
            else {
                done = true;
            }
        }

        Ok(())
    }
}
