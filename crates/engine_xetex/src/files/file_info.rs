//! The file-info primitives: `\filesize`, `\filedump`, `\filemoddate` and `\mdfivesum`.

use crate::base::consts::INVALID_HANDLE;
use crate::base::units::StrNumber;
use crate::ffi::bridge::{
    FileFormat, get_file_md5, input_get_mtime, input_get_size, input_open, input_read,
};
use crate::ffi::clock::{make_pdf_time, nul_terminated};
use crate::files::io::{input_close, input_seek};
use crate::print::errors::Abort;
use crate::tables::string_pool::StringsState;

const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

impl StringsState {
    fn open_tex_file(&self, s: StrNumber) -> usize {
        input_open(&self.texstring_bytes(s), FileFormat::Tex, false)
    }

    pub(crate) fn file_size(&mut self, s: StrNumber) -> Result<(), Abort> {
        let handle = self.open_tex_file(s);
        if handle == INVALID_HANDLE {
            return Ok(());
        }

        let file_len = input_get_size(handle);
        if input_close(handle) {
            return Err(Abort::InputClose);
        }

        self.push_bytes_or_fill((file_len as u64).to_string().as_bytes());
        Ok(())
    }

    pub(crate) fn file_dump(
        &mut self,
        s: StrNumber,
        offset: libc::c_int,
        length: libc::c_int,
    ) -> Result<(), Abort> {
        if length == 0 {
            return Ok(());
        }

        if self.pool_ptr + 2 * length + 1 >= self.pool_size {
            self.pool_ptr = self.pool_size;
            return Ok(());
        }

        let mut buffer = vec![0u8; length as usize + 1];
        let handle = self.open_tex_file(s);
        if handle == INVALID_HANDLE {
            return Ok(());
        }

        input_seek(handle, offset as u64);
        let actual = if input_read(handle, &mut buffer[..length as usize]) {
            length as usize
        } else {
            0
        };
        if input_close(handle) {
            return Err(Abort::InputClose);
        }

        self.push_hex(&buffer[..actual]);
        Ok(())
    }

    pub(crate) fn file_mod_date(&mut self, s: StrNumber) -> Result<(), Abort> {
        let handle = self.open_tex_file(s);
        if handle == INVALID_HANDLE {
            return Ok(());
        }

        let mtime = input_get_mtime(handle);
        if input_close(handle) {
            return Err(Abort::InputClose);
        }

        let mut buf = [0u8; 30];
        make_pdf_time(mtime, &mut buf, true);
        self.push_bytes_or_fill(nul_terminated(&buf));
        Ok(())
    }

    /// Appends the hex MD5 of file (or string) `s` to the pool.
    pub(crate) fn md5_sum(&mut self, s: StrNumber, file: bool) {
        let xname = self.texstring_bytes(s);
        let digest = if file {
            get_file_md5(&xname)
        } else {
            Some(texyon_bridge_core::data_md5(&xname))
        };
        let Some(digest) = digest else {
            return;
        };
        if self.pool_ptr + 2 * digest.len() as i32 >= self.pool_size {
            return;
        }
        self.push_hex(&digest);
    }

    fn push_hex(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push_bytes(&[
                HEX_DIGITS[(b >> 4) as usize],
                HEX_DIGITS[(b & 0xF) as usize],
            ]);
        }
    }
}
