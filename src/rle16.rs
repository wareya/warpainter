use std::io::{Write, Read, Result};

pub struct Compressor<W : Write>
{
    inner : W,
}

impl<W : Write> Compressor<W>
{
    pub fn new(inner : W) -> Self {
        Self { inner }
    }
}
impl<W : Write> Write for Compressor<W>
{
    fn write(&mut self, buf : &[u8]) -> Result<usize>
    {
        let mut i = 0;
        let len = buf.len();
        
        let mut lit_start = !0usize;
        
        macro_rules! check_flush { () =>
        {{
            if lit_start != !0usize
            {
                let run_len = i - lit_start;
                if run_len > 0
                {
                    //assert!(run_len <= 127);
                    //assert!(run_len > 0);
                    self.inner.write_all(&[run_len as u8])?;
                    self.inner.write_all(&buf[lit_start..i])?;
                    lit_start = !0usize;
                }
            }
        }}}
        
        // deliberately leave last 8 bytes so we can safely read ahead
        // we'll finish up with RLE disabled, which doesn't need to read ahead
        while i + 32 < len
        {
            // from_ne_bytes etc. do not care about alignment. on unaligned platforms like ARM they are just slower but still safe.
            // unwrap: because of the above bounds check, this will not panic
            if u128::from_ne_bytes(buf[i..i+16].try_into().unwrap())
                == u128::from_ne_bytes(buf[i+16..i+32].try_into().unwrap())
            {
                check_flush!();
                let end = (len - 15).min(i + 127*16);
                let mut j = i+16;
                // unwrap: because of the j < end check, this will not panic
                while j < end
                    && u128::from_ne_bytes(buf[i..i+16].try_into().unwrap())
                        == u128::from_ne_bytes(buf[j..j+16].try_into().unwrap())
                {
                    j = j+16;
                }
                let run_len = (j - i) / 16;
                //assert!(run_len <= 127);
                //assert!(run_len > 0);
                self.inner.write_all(&[!(run_len as u8)])?;
                self.inner.write_all(&buf[i..i+16])?;
                i = j;
            }
            else
            {
                if lit_start != !0usize && i - lit_start >= 127
                {
                    check_flush!();
                }
                if lit_start == !0usize
                {
                    lit_start = i;
                }
                i = i+1;
            }
        }
        // continue with RLE logic disabled
        while i < len
        {
            if lit_start != !0usize && i - lit_start >= 127
            {
                check_flush!();
            }
            if lit_start == !0usize
            {
                lit_start = i;
            }
            i = i+1;
        }
        // flush any trailing literal
        #[allow(unused_assignments)]
        {
            check_flush!();
        }
        
        Ok(len)
    }

    fn flush(&mut self) -> Result<()>
    {
        self.inner.flush()
    }
}


pub struct Decompressor<R : Read>
{
    inner : R,
    cmd : Vec<u8>, // decompressed current command
    pos : usize, // number of bytes processed from current command
}

impl<R : Read> Decompressor<R>
{
    pub fn new(inner : R) -> Self
    {
        Self { inner, cmd : Vec::new(), pos : 0 }
    }
    fn check_cmd(&mut self) -> Result<()>
    {
        let cmd = &mut self.cmd;
        if self.pos == cmd.len()
        {
            cmd.clear();
            let mut word = [0u8; 1];
            let r = self.inner.read(&mut word)?;
            if r == 0
            {
                self.cmd.clear();
                self.pos = !0usize;
                return Ok(());
            }
            else if word[0] > 127
            {
                let len = (!word[0]) as usize;
                cmd.resize(len * 16, 0);
                let mut word = [0u8; 16];
                self.inner.read_exact(&mut word)?;
                for part in cmd.chunks_exact_mut(16)
                {
                    part.copy_from_slice(&word);
                }
            }
            else
            {
                let len = word[0] as usize;
                cmd.resize(len, 0);
                self.inner.read_exact(&mut cmd[..])?;
            }
            self.pos = 0;
        }
        Ok(())
    }
}

impl<R : Read> Read for Decompressor<R>
{
    fn read(&mut self, out : &mut [u8]) -> Result<usize>
    {
        let mut written = 0;
        while written < out.len()
        {
            self.check_cmd()?;
            if self.pos == !0usize
            {
                return Ok(written);
            }
            
            let avail = self.cmd.len() - self.pos;
            //assert!(avail > 0);
            
            let to_copy = avail.min(out.len() - written);
            
            //assert!(self.pos + to_copy <= self.cmd.len());
            //assert!(written + to_copy <= out.len());
            
            out[written..written + to_copy].copy_from_slice(&self.cmd[self.pos..self.pos + to_copy]);
            
            self.pos += to_copy;
            written += to_copy;
        }
        
        Ok(written)
    }
}
