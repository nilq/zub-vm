use super::*;
use gc::trace::{ Trace, Tracer };

#[derive(Debug, Clone)]
pub struct Chunk {
    code: Vec<u8>,
    name: String,
    constants: Vec<Value>,
    lines: Vec<Line>,
}

impl Trace<Object> for Chunk {
    fn trace(&self, tracer: &mut Tracer<Object>) {
        self.constants.trace(tracer);
    }
}

#[derive(Debug, Copy, Clone)]
struct Line {
    pub start: usize,
    pub line: usize,
}

impl Chunk {
    pub fn new(name: String) -> Self {
        Chunk {
            code: Vec::new(),
            name,
            constants: Vec::new(),
            lines: Vec::new()
        }
    }

    pub fn write(&mut self, op: Op, line: usize) {
        self.add_line(line);
        op.write(&mut self.code);
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn write_byte_at(&mut self, idx: usize, byte: u8) {
        self.code[idx] = byte;
    }

    pub fn write_u64(&mut self, val: u64) {
        (0..8).for_each(|i| self.write_byte(((val >> i * 8) & 0xFF) as u8))
    }

    #[inline]
    pub fn add_constant(&mut self, constant: Value) -> u8 {
        for (i, c) in self.constants.iter().enumerate() {
            if *c == constant {
                return i as u8;
            }
        }

        if self.constants.len() == 1028 {
            panic!("A chunk cannot have more than 1028 constants");
        }

        self.constants.push(constant);
        self.constants.len() as u8 - 1
    }

    #[inline]
    pub fn string_constant(&mut self, heap: &mut Heap<Object>, string: &str) -> u8 {
        for (i, c) in self.constants().enumerate() {
            let obj = c
                .as_object()
                .and_then(|o| heap.get(o))
                .and_then(|o| o.as_string());

            if let Some(s) = obj {
                if s == string {
                    return i as u8
                }
            }
        }

        let handle = heap.insert(Object::String(string.to_owned())).into_handle();
        self.add_constant(handle.into())
    }

    pub fn constants(&self) -> Constants {
        Constants::new(self.constants.iter())
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    fn add_line(&mut self, line: usize) {
        match self.lines.last().cloned() {
            Some(last) if last.line >= line => return,
            _ => (),
        }

        self.lines.push(Line {
            start: self.code.len(),
            line: line,
        });
    }

    #[inline]
    pub fn get(&self, ip: usize) -> u8 {
        self.code[ip]
    }

    #[inline]
    pub fn get_constant(&self, idx: u8) -> Option<&Value> {
        self.constants.get(idx as usize)
    }

    pub fn line(&self, offset: usize) -> usize {
        let idx =
            self.lines
                .binary_search_by_key(&offset, |line_info| line_info.start)
                .map_err(|idx| idx - 1) // on failure we want the earlier line
                .unwrap_or_else(|idx| idx);
        self.lines[idx].line
    }

    #[inline]
    pub fn read_byte(&self, idx: usize) -> u8 {
        self.code[idx]
    }

    #[inline]
    pub fn read_u16(&self, idx: usize) -> u16 {
        let mut t = 0u16;
        let size = ::std::mem::size_of::<u16>();
        
        unsafe {
            ::std::ptr::copy_nonoverlapping(
                &self.code[idx],
                &mut t as *mut u16 as *mut u8,
                size);
        }

        t.to_le()
    }

    #[inline]
    pub fn read_u64(&self, idx: usize) -> u64 {
        let mut t = 0u64;
        let size = ::std::mem::size_of::<u64>();
        
        unsafe {
            ::std::ptr::copy_nonoverlapping(
                &self.code[idx],
                &mut t as *mut u64 as *mut u8,
                size);
        }

        t.to_le()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub struct Constants<'c> {
    iter: ::std::slice::Iter<'c, Value>
}

impl<'c> Constants<'c> {
    fn new(iter: ::std::slice::Iter<'c, Value>) -> Self {
        Constants { iter }
    }
}

impl<'c> Iterator for Constants<'c> {
    type Item = Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|v| *v)
    }
}

impl AsRef<[u8]> for Chunk {
    fn as_ref(&self) -> &[u8] {
        &self.code[..]
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Op {
    Return,
    Constant(u8),
    Nil,
    True,
    False,
    Pop,
    GetLocal,
    SetLocal,
    GetGlobal,
    DefineGlobal,
    SetGlobal,
    GetUpValue,
    SetUpValue,

    Equal,
    Less,
    Greater,

    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,

    Not,
    Neg,

    Print,
    Jump,
    JumpIfFalse,
    Loop,
    Immediate,
    
    Call(u8),
    Closure,
    CloseUpValue,

    List,
    Dict,
    GetElement,
    SetElement,
}

impl Op {
    fn write(&self, buf: &mut Vec<u8>) {
        use self::Op::*;

        match *self {
            Return => buf.push(0x00),
            Constant(idx) => { buf.push(0x01); buf.push(idx); }
            Print => buf.push(0x02),
            Add => buf.push(0x03),
            Sub => buf.push(0x04),
            Mul => buf.push(0x05),
            Div => buf.push(0x06),
            Not => buf.push(0x07),
            Neg => buf.push(0x08),
            Equal => buf.push(0x09),
            Greater => buf.push(0x0a),
            Less => buf.push(0x0b),
            Jump => buf.push(0x0c),
            JumpIfFalse => buf.push(0x0d),
            Pop => buf.push(0x0e),
            GetGlobal => buf.push(0x0f),
            SetGlobal => buf.push(0x10),
            GetLocal => buf.push(0x11),
            SetLocal => buf.push(0x12),
            Immediate => buf.push(0x13),
            Nil => buf.push(0x14),
            True => buf.push(0x15),
            False => buf.push(0x16),
            Call(a) => buf.push(0x17 + a),
            Loop => buf.push(0x20),
            CloseUpValue => buf.push(0x21),
            GetUpValue => buf.push(0x22),
            SetUpValue => buf.push(0x23),
            Closure => buf.push(0x24),
            DefineGlobal => buf.push(0x25),

            List => buf.push(0x26),
            Rem => buf.push(0x27),
            Dict => buf.push(0x28),
            SetElement => buf.push(0x29),
            GetElement => buf.push(0x30),
            Pow => buf.push(0x31),
        }
    }
}

macro_rules! decode_op {
    ($op:expr, $this:ident) => {
        match $op {
            0x00 => $this.ret(),
            0x01 => { let idx = $this.read_byte(); $this.constant(idx); }
            0x02 => $this.print(),
            0x03 => $this.add(),
            0x04 => $this.sub(),
            0x05 => $this.mul(),
            0x06 => $this.div(),
            0x07 => $this.not(),
            0x08 => $this.neg(),
            0x09 => $this.eq(),
            0x0a => $this.gt(),
            0x0b => $this.lt(),
            0x0c => $this.jmp(),
            0x0d => $this.jze(),
            0x0e => { $this.pop(); },
            0x0f => $this.get_global(),
            0x10 => $this.set_global(),
            0x11 => $this.get_local(),
            0x12 => $this.set_local(),
            0x13 => $this.immediate(),
            0x14 => $this.imm_nil(),
            0x15 => $this.imm_true(),
            0x16 => $this.imm_false(),
            a @ 0x17..=0x1f => {
                $this.call(a - 0x17)
            },
            0x20 => $this.op_loop(),
            0x21 => $this.close_upvalue(),
            0x22 => $this.get_upvalue(),
            0x23 => $this.set_upvalue(),
            0x24 => $this.closure(),
            0x25 => $this.define_global(),
            0x26 => $this.list(),
            0x27 => $this.rem(),
            0x28 => $this.dict(),
            0x29 => $this.set_element(),
            0x30 => $this.get_element(),
            0x31 => $this.pow(),
            _ => {
                panic!("Unknown op {}", $op);
            }
        }
    }
}