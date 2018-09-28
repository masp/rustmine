extern crate bytes;

use std::io::{Error, Read, ErrorKind, Result, BufReader, Cursor};
use bytes::*;

macro_rules! define_read_var {
    ($wrapper_name:ident, $typ:ty, $maxb:expr) => {
        pub fn $wrapper_name(bytes: &mut Cursor<Bytes>) -> Result<($typ, usize)> {
            let mut num_read: usize = 0;
            let mut result: $typ = 0;
            loop {
                if num_read >= $maxb {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Variable number at position {} has an invalid format (too many bytes)",
                                    bytes.position())));
                }
                let read = bytes.read_u8()?;
                let value = read & 0b01111111;
                result |= (value as $typ) << (7 * num_read);

                num_read += 1;

                if (read & 0b10000000) == 0 {
                    break;
                }
            }


            Ok((result, num_read))
        }
    };
}

define_read_var!(read_var_int, i32, 5);
define_read_var!(read_var_long, i64, 10);


pub trait MinecraftBufRead {
    fn read_var_int(&mut self) -> Result<(i32, usize)>;
    fn read_var_long(&mut self) -> Result<(i64, usize)>;
}

impl MinecraftBufRead for Cursor<Bytes> {
    fn read_var_int(&mut self) -> Result<(i32, usize)> {
        read_var_int(self)
    }

    fn read_var_long(&mut self) -> Result<(i64, usize)> {
        read_var_long(self)
    }
}

// This all is copied over from io::Read, with WouldBlock ignored rather than causing an error to work with mio
struct Guard<'a> { buf: &'a mut Vec<u8>, len: usize }

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        unsafe { self.buf.set_len(self.len); }
    }
}

pub fn read_to_end<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize> {
    let start_len = buf.len();
    let mut g = Guard { len: buf.len(), buf: buf };
    let ret;
    loop {
        if g.len == g.buf.len() {
            unsafe {
                g.buf.reserve(32);
                let capacity = g.buf.capacity();
                g.buf.set_len(capacity);
                r.initializer().initialize(&mut g.buf[g.len..]);
            }
        }

        match r.read(&mut g.buf[g.len..]) {
            Ok(0) => {
                ret = Ok(g.len - start_len);
                break;
            }
            Ok(n) => g.len += n,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => { // This is changed
                println!("Would block");
                ret = Ok(g.len - start_len);
                break;
            }
            Err(e) => {
                ret = Err(e);
                break;
            }
        }
    }

    ret
}