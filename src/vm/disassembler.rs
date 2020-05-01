use super::*;
use gc::trace::{ Trace, Tracer };
use colored::Colorize;

pub struct Disassembler<'c> {
    offset: usize,
    line: usize,
    chunk: &'c Chunk,
    heap: &'c Heap<Object>,
}

impl<'c> Disassembler<'c> {
    pub fn new(chunk: &'c Chunk, heap: &'c Heap<Object>) -> Self {
        Disassembler {
            offset: 0,
            line: 0,
            chunk,
            heap,
        }
    }

    pub fn disassemble(mut self) {
        let bytes = self.chunk.as_ref();

        println!();
        let name = format!("== {} ==", self.chunk.name());
        eprint!("{}", name.cyan());

        while self.offset < bytes.len() {
            self.disassemble_instruction();
        }
    }

    fn disassemble_instruction(&mut self) {
        let line = self.chunk.line(self.offset);
        if self.line == line {
        } else {
            self.line = line;
        }
        let inst = self.read_byte();
        println!();
        let off = format!("{:04} | ", self.offset);

        eprint!("{}", off.blue());
        decode_op!(inst, self);
    }

    fn constant(&mut self, idx: u8) {
        let val = self.chunk.get_constant(idx).expect("invalid constant segment index");
        eprint!("OP_CONSTANT\t{}\t{:?}", idx, val);
    }

    fn ret(&self) { eprint!("OP_RETURN"); }
    fn print(&self) { eprint!("OP_PRINT"); }
    fn add(&self) { eprint!("OP_ADD"); }
    fn sub(&self) { eprint!("OP_SUB"); }
    fn mul(&self) { eprint!("OP_MUL"); }
    fn div(&self) { eprint!("OP_DIV"); }
    fn neg(&self) { eprint!("OP_NEG"); }
    fn not(&self) { eprint!("OP_NOT"); }
    fn eq(&self) { eprint!("OP_EQ"); }
    fn gt(&self) { eprint!("OP_GT"); }
    fn lt(&self) { eprint!("OP_LT"); }
    fn pop(&self) { eprint!("OP_POP"); }

    fn list(&mut self) {}
    fn get_element(&mut self) {}
    fn set_element(&mut self) {}

    fn jmp(&mut self) {
        let offset = self.offset - 1;
        let ip = self.read_u16();
        eprint!("OP_JUMP\t{} -> {}", offset, ip);
    }

    fn jze(&mut self) {
        let offset = self.offset - 1;
        let ip = self.read_u16();
        eprint!("OP_JUMP_IF_FALSE\t{} -> {}", offset, ip);
    }

    fn op_loop(&mut self) {
        let sub = self.read_u16() as usize;
        eprint!("OP_LOOP\t{} -> {}", self.offset, self.offset - sub);
    }

    fn get_global(&mut self) {
        let val = self.read_constant();
        eprint!("OP_GET_GLOBAL\t{}", val.with_heap(self.heap));
    }

    fn set_global(&mut self) {
        let val = self.read_constant();
        eprint!("OP_SET_GLOBAL\t{}", val.with_heap(self.heap));
    }

    fn define_global(&mut self) {
        let val = self.read_constant();
        eprint!("OP_DEFINE_GLOBAL\t{}", val.with_heap(self.heap));
    }

    fn get_local(&mut self) {
        let val = self.read_byte();
        eprint!("OP_GET_LOCAL\t{}", val);
    }

    fn set_local(&mut self) {
        let val = self.read_byte();
        eprint!("OP_SET_LOCAL\t{}", val);
    }

    fn immediate(&mut self) {
        self.offset += 8;
        let b1 = self.chunk.get(self.offset - 8) as u64;
        let b2 = self.chunk.get(self.offset - 7) as u64;
        let b3 = self.chunk.get(self.offset - 6) as u64;
        let b4 = self.chunk.get(self.offset - 5) as u64;
        let b5 = self.chunk.get(self.offset - 4) as u64;
        let b6 = self.chunk.get(self.offset - 3) as u64;
        let b7 = self.chunk.get(self.offset - 2) as u64;
        let b8 = self.chunk.get(self.offset - 1) as u64;
        let raw = b1 +
            (b2 << 8) +
            (b3 << 16) +
            (b4 << 24) +
            (b5 << 32) +
            (b6 << 40) +
            (b7 << 48) +
            (b8 << 56);
        let val = unsafe { Value::from_raw(raw) };
        eprint!("OP_FLOAT\t{}", val.with_heap(self.heap));
    }

    fn imm_nil(&self) {
        eprint!("OP_NIL");
    }

    fn imm_true(&self) {
        eprint!("OP_TRUE");
    }

    fn imm_false(&self) {
        eprint!("OP_FALSE");
    }

    fn call(&self, arity: u8) {
        eprint!("OP_CALL_{}", arity);
    }

    fn invoke(&mut self, arity: u8) {
        let idx = self.read_byte();
        let val = self.chunk.get_constant(idx).expect("invalid constant segment index");
        eprint!("OP_INVOKE_{} {}", arity, val.with_heap(&self.heap));
    }

    fn close_upvalue(&self) {
        eprint!("OP_CLOSE_UPVALUE");
    }

    fn get_upvalue(&mut self) {
        let index = self.read_byte();
        eprint!("OP_GET_UPVALUE\t{}", index);
    }

    fn set_upvalue(&mut self) {
        let index = self.read_byte();
        eprint!("OP_SET_UPVALE\t{}", index);
    }

    fn closure(&mut self) {
        let val = self.read_constant();
        let count = val
            .as_object()
            .and_then(|o| self.heap.get(o))
            .and_then(|o| o.as_function())
            .expect("closure argument to be a function")
            .upvalue_count();

        print!("OP_CLOSURE\t{} ", val.with_heap(self.heap));
        println!();

        if let Variant::Obj(cl) = val.with_heap(self.heap).item.decode() {
            unsafe {
                let closure = cl.get_unchecked().as_function().unwrap();

                let dis = Disassembler::new(closure.chunk(), &self.heap);
                dis.disassemble()
            }
        }

        println!();
        println!();

        for _ in 0..count {
            let is_local = self.read_byte() > 0;
            let index = self.read_byte();
            if is_local {
                print!("L{}", index);
            } else {
                print!("U{}", index);
            }
        }
        eprintln!();
    }

    fn class(&mut self, idx: u8) {
        let val = self.chunk.get_constant(idx).expect("invalid constant segment index");
        let methods = self.read_byte();
        eprint!("OP_CLASS\t{}\t{}\t({} method(s))", idx, val.with_heap(&self.heap), methods);
    }

    fn get_property(&mut self) {
        let idx = self.read_byte();
        let val = self.chunk.get_constant(idx).expect("invalid constant segment index");
        eprint!("GET_PROPERTY\t{}\t{}", idx, val.with_heap(&self.heap));
    }

    fn set_property(&mut self) {
        let idx = self.read_byte();
        let val = self.chunk.get_constant(idx).expect("invalid constant segment index");
        eprint!("SET_PROPERTY\t{}\t{}", idx, val.with_heap(&self.heap));
    }

    fn read_byte(&mut self) -> u8 {
        self.offset += 1;
        self.chunk.as_ref()[self.offset - 1]
    }

    fn read_u16(&mut self) -> u16 {
        self.offset += 2;
        let lo = self.chunk.get(self.offset - 2) as u16;
        let hi = self.chunk.get(self.offset - 1) as u16;
        lo + (hi << 8)
    }

    fn read_constant(&mut self) -> Value {
        let idx = self.read_byte();
        *self.chunk.get_constant(idx).expect("invalid constant segment index")
    }
}
