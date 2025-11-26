use anyhow::{Ok, Result, bail, ensure};
use bytes::Buf;

#[derive(Debug, PartialEq)]
pub enum Constant {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
}

#[derive(Debug, PartialEq)]
pub struct LocVar {
    varname: String,
    startpc: u32,
    endpc: u32,
}

#[derive(Debug, PartialEq)]
pub struct Function {
    source: String,
    line_defined: u32,
    last_line_defined: u32,
    nups: u8,
    num_params: u8,
    is_vararg: u8,
    maxstacksize: u8,
    code: Vec<u32>,
    constants: Vec<Constant>,
    funs: Vec<Function>,
    lineinfo: Vec<u32>,
    locvars: Vec<LocVar>,
    upvalues: Vec<String>,
}

trait LuacBuf: Buf {
    fn get_string(&mut self) -> Result<String> {
        ensure!(self.remaining() >= 8, "truncated string length");
        let len = self.get_u64_le().try_into()?;
        ensure!(self.remaining() >= len, "truncated string contents");
        let str = if len == 0 {
            "".to_owned()
        } else {
            String::from_utf8_lossy(self.take(len - 1).chunk()).to_string()
        };
        self.advance(len);
        Ok(str)
    }
    fn get_function(&mut self) -> Result<Function> {
        let source = self.get_string()?;
        ensure!(self.remaining() >= 16, "truncated function header");
        let line_defined = self.get_u32_le();
        let last_line_defined = self.get_u32_le();
        let nups = self.get_u8();
        let num_params = self.get_u8();
        let is_vararg = self.get_u8();
        let maxstacksize = self.get_u8();
        let codelen = self.get_u32_le().try_into()?;
        ensure!(
            self.remaining() >= codelen * 4 + 4,
            "truncated function code"
        );
        let mut code = Vec::with_capacity(codelen);
        for _ in 0..codelen {
            code.push(self.get_u32_le());
        }
        let constlen = self.get_u32_le().try_into()?;
        let mut constants = Vec::with_capacity(constlen);
        for _ in 0..constlen {
            ensure!(self.remaining() >= 1, "truncated constants");
            let ttype = self.get_u8();
            constants.push(match ttype {
                0 => Ok(Constant::Nil),
                1 => {
                    ensure!(self.remaining() >= 1);
                    Ok(Constant::Boolean(self.get_u8() != 0))
                }
                3 => {
                    ensure!(self.remaining() >= 8);
                    Ok(Constant::Number(self.get_f64_le()))
                }
                4 => Ok(Constant::String(self.get_string()?)),
                _ => bail!("invalid constant type {}", ttype),
            }?);
        }
        ensure!(self.remaining() >= 4, "truncated functions");
        let funlen = self.get_u32_le().try_into()?;
        let mut funs = Vec::with_capacity(funlen);
        for _ in 0..funlen {
            funs.push(self.get_function()?);
        }
        ensure!(self.remaining() >= 4, "truncated debug lineinfo size");
        let sizelineinfo = self.get_u32_le().try_into()?;
        ensure!(
            self.remaining() >= 4 * sizelineinfo,
            "truncated debug lineinfo"
        );
        let mut lineinfo = Vec::with_capacity(sizelineinfo);
        for _ in 0..sizelineinfo {
            lineinfo.push(self.get_u32_le());
        }
        ensure!(self.remaining() >= 4, "truncated debug locvars size");
        let sizelocvars = self.get_u32_le().try_into()?;
        let mut locvars = Vec::with_capacity(sizelocvars);
        for _ in 0..sizelocvars {
            let varname = self.get_string()?;
            ensure!(self.remaining() >= 8, "truncated debug locvars");
            let startpc = self.get_u32_le();
            let endpc = self.get_u32_le();
            locvars.push(LocVar {
                varname,
                startpc,
                endpc,
            });
        }
        ensure!(self.remaining() >= 4, "truncated debug upvalues size");
        let sizeupvalues = self.get_u32_le().try_into()?;
        let mut upvalues = Vec::with_capacity(sizeupvalues);
        for _ in 0..sizeupvalues {
            upvalues.push(self.get_string()?);
        }
        let fun = Function {
            source,
            line_defined,
            last_line_defined,
            nups,
            num_params,
            is_vararg,
            maxstacksize,
            code,
            constants,
            funs,
            lineinfo,
            locvars,
            upvalues,
        };
        Ok(fun)
    }
}
impl LuacBuf for &[u8] {}

pub fn undump(data: &[u8]) -> Result<Function> {
    let mut p = data;
    ensure!(p.remaining() >= 12, "truncated header");
    ensure!(p.get_u32().to_be_bytes() == *b"\x1bLua", "bad signature");
    ensure!(p.get_u8() == 0x51, "bad luac version");
    ensure!(p.get_u8() == 0x0, "bad luac format");
    ensure!(p.get_u8() == 0x1, "bad endianness");
    ensure!(p.get_u8() == 0x4, "bad sizeof(int)");
    ensure!(p.get_u8() == 0x8, "bad sizeof(size_t)");
    ensure!(p.get_u8() == 0x4, "bad sizeof(Instruction)");
    ensure!(p.get_u8() == 0x8, "bad sizeof(lua_Number)");
    ensure!(p.get_u8() == 0x0, "lua_Number must be floating point");
    let fun = p.get_function()?;
    ensure!(!p.has_remaining(), "extraneous bytes ({})", p.remaining());
    Ok(fun)
}

#[cfg(test)]
mod tests {
    use crate::undump::*;

    #[test]
    fn test() {
        let return42hello = b"\
\x1b\x4c\x75\x61\x51\x00\x01\x04\x08\x04\x08\x00\x09\x00\x00\x00\
\x00\x00\x00\x00\x40\x77\x61\x74\x2e\x6c\x75\x61\x00\x00\x00\x00\
\x00\x00\x00\x00\x00\x00\x00\x02\x02\x04\x00\x00\x00\x01\x00\x00\
\x00\x41\x40\x00\x00\x1e\x00\x80\x01\x1e\x00\x80\x00\x02\x00\x00\
\x00\x03\x00\x00\x00\x00\x00\x00\x45\x40\x04\x06\x00\x00\x00\x00\
\x00\x00\x00\x68\x65\x6c\x6c\x6f\x00\x00\x00\x00\x00\x04\x00\x00\
\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\x00\x01\x00\x00\
\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert_eq!(
            undump(return42hello).unwrap(),
            Function {
                source: "@wat.lua".to_owned(),
                line_defined: 0,
                last_line_defined: 0,
                nups: 0,
                num_params: 0,
                is_vararg: 2,
                maxstacksize: 2,
                code: vec![1, 16449, 25165854, 8388638],
                constants: vec![Constant::Number(42.0), Constant::String("hello".to_owned())],
                funs: vec![],
                lineinfo: vec![1, 1, 1, 1],
                locvars: vec![],
                upvalues: vec![],
            }
        )
    }
}
