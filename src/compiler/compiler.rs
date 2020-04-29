use super::chunk::{ Chunk, Op };
use super::*;

#[derive(Debug)]
struct Local {
    pub name: String,
    pub depth: usize,
    pub captured: bool,
    pub reserved: bool,
}

#[derive(Debug, Clone)]
struct UpValue {
    pub index: u8,
    pub is_local: bool,
}

#[derive(Debug)]
struct CompileState {
    line: usize,
    locals: Vec<Local>,
    upvalues: Vec<UpValue>,
    function: FunctionBuilder,
    scope_depth: usize,
    breaks: Vec<usize>,
    method: bool
}

impl CompileState {
    pub fn new(method: bool, reserved: &str, function: FunctionBuilder, scope_depth: usize) -> Self {
        let locals = vec![
            Local {
                name: reserved.into(),
                depth: 1,
                captured: false,
                reserved: true
            }
        ];

        CompileState {
            line: 0,
            locals,
            upvalues: Vec::new(),
            function,
            scope_depth,
            breaks: Vec::new(),
            method,
        }
    }

    fn capture_local(&mut self, var: &str) -> Option<u8> {
        for (i, local) in self.locals.iter_mut().enumerate().rev() {
            if local.name == var {
                local.captured = true;

                return Some(i as u8)
            }
        }

        None
    }

    fn add_local(&mut self, var: &str, depth: usize) -> u8 {
        let depth = self.scope_depth - depth;
        
        if self.locals.len() == std::u8::MAX as usize {
            panic!("local variable overflow")
        }

        self.locals.push(
            Local {
                name: var.into(),
                depth,
                captured: false,
                reserved: false,
            }
        );

        (self.locals.len() - 1) as u8
    }

    fn resolve_local(&mut self, var: &str) -> u8 {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == var {
                return i as u8
            }
        }
        
        panic!("TODO: unresolved var")
    }

    fn add_upvalue(&mut self, index: u8, is_local: bool) -> u8 {
        for (i, upval) in self.upvalues.iter().enumerate() {
            if upval.index == index && upval.is_local == is_local {
                return i as u8
            }
        }

        if self.upvalues.len() == std::u8::MAX as usize {
            panic!("too many upvalues, not cool")
        } else {
            self.upvalues.push(
                UpValue {
                    index,
                    is_local
                }
            );

            (self.upvalues.len() - 1) as u8
        }
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        let last = self.scope_depth;
        
        self.scope_depth -= 1;

        let mut ops = Vec::new();

        self.locals.retain(|local| {
            if local.depth < last || local.reserved {
                return true
            }

            if local.captured {
                ops.push(Op::CloseUpValue)
            } else {
                ops.push(Op::Pop)
            }

            false
        });

        ops.into_iter().rev().for_each(|op| self.emit(op))
    }

    fn emit(&mut self, op: Op) {
        self.function.chunk_mut().write(op, self.line);
    }

    fn add_break(&mut self, jmp: usize) {
        self.breaks.push(jmp);
    }

    fn breaks(&mut self) -> Vec<usize> {
        let bs = self.breaks.clone();
        self.breaks.clear();

        bs
    }
}


pub struct Compiler<'g> {
    heap: &'g mut Heap<Object>,
    states: Vec<CompileState>,
}

impl<'g> Compiler<'g> {
    pub fn new(heap: &'g mut Heap<Object>) -> Self {
        Compiler {
            heap,
            states: Vec::new(),
        }
    }

    pub fn compile(mut self, atoms: &[Atom]) -> Function {
        self.start_function(false, "<zub>", 0, 0);

        for atom in atoms.iter() {
            self.compile_atom(atom)
        }

        self.end_function()
    }

    fn compile_atom(&mut self, atom: &Atom) {
        use self::Atom::*;

        match atom {
            Literal(ref lit) => self.emit_constant(lit),
            Unary(ref op, ref node) => {
                self.compile_atom(node.inner());

                use self::UnaryOp::*;

                match op {
                    Neg => self.emit(Op::Neg),
                    Not => self.emit(Op::Not)
                }
            },

            Var(ref var) => self.var_get(var),
            Mutate(ref lhs, ref rhs) => {
                // Currently just handling Var
                if let Var(ref var) = lhs.inner() {
                    self.compile_atom(rhs.inner());

                    if var.is_upvalue() {
                        let idx = self.resolve_upvalue(var.name());
                        
                        self.emit(Op::SetUpValue);
                        self.emit_byte(idx)
                    } else {
                        if var.depth.is_none() { // Global
                            self.set_global(var.name())
                        } else {
                            let idx = self.state_mut().resolve_local(var.name());

                            self.emit(Op::SetLocal);
                            self.emit_byte(idx)
                        }
                    }
                } else {
                    // When classes are a thing, this is where we handle setting properties
                    panic!("can't mutate non-variable")
                }
            },

            Return(val) => self.emit_return((*val).clone()),

            Binary(lhs, op, rhs) => {
                use self::BinaryOp::*;
                
                match op {
                    And => {
                        self.compile_atom(lhs.inner());

                        let short_circuit_jmp = self.emit_jze();

                        self.emit(Op::Pop);
                        self.compile_atom(rhs.inner());

                        self.patch_jmp(short_circuit_jmp);
                    },

                    Or => {
                        self.compile_atom(lhs.inner());

                        let else_jmp = self.emit_jze();
                        let end_jmp = self.emit_jmp();

                        self.patch_jmp(else_jmp);
                        self.emit(Op::Pop);

                        self.compile_atom(rhs.inner());
                        
                        self.patch_jmp(end_jmp)
                    },

                    _ => {
                        // This looks kinda funny, but it's an ok way of matching I guess

                        self.compile_atom(lhs.inner()); // will handle type in the future :)
                        self.compile_atom(rhs.inner());

                        match op {
                            Add => self.emit(Op::Add),
                            Sub => self.emit(Op::Sub),
                            Mul => self.emit(Op::Mul),
                            Div => self.emit(Op::Div),

                            Equal => self.emit(Op::Equal),
                            Gt => self.emit(Op::Greater),
                            Lt => self.emit(Op::Less),

                            GtEqual => {
                                self.emit(Op::Less);
                                self.emit(Op::Not)
                            },

                            LtEqual => {
                                self.emit(Op::Greater);
                                self.emit(Op::Not)
                            },

                            NEqual => {
                                self.emit(Op::Equal);
                                self.emit(Op::Not)
                            },

                            _ => {}
                        }
                    }
                }
            },

            Bind(ref var, ref init) => {
                self.compile_atom(init.inner());
                self.var_define(var, None)
            },

            _ => todo!()
        }
    }



    fn var_get(&mut self, var: &Binding) {
        if var.is_upvalue() {
            let idx = self.resolve_upvalue(var.name());

            self.emit(Op::GetUpValue);
            self.emit_byte(idx);
        } else {
            // local time B)
            if var.depth.is_none() {
                self.emit(Op::GetGlobal);
                let idx = self.string_constant(var.name());
                self.emit_byte(idx)
            } else {
                let idx = self.state_mut().resolve_local(var.name());

                self.emit(Op::GetLocal);
                self.emit_byte(idx)
            }
        }
    }

    fn var_define(&mut self, var: &Binding, constant: Option<u8>) {
        // If there's depth, it's a local
        if let Some(depth) = var.depth {
            self.state_mut().add_local(var.name(), depth);
            self.state_mut().resolve_local(var.name());
        } else {
            self.emit(Op::DefineGlobal);

            let idx = constant.unwrap_or_else(|| {
                self.string_constant(var.name())
            });

            self.emit_byte(idx)
        }
    }

    fn set_global(&mut self, name: &str) {
        self.emit(Op::SetGlobal);

        let idx = {
            let chunk = self.states.last_mut()
                .unwrap()
                .function
                .chunk_mut();
            
            chunk.string_constant(self.heap, name)
        };

        self.emit_byte(idx)
    }

    fn function_decl(&mut self, f: &IrFunction) {
        let name = f.var.name();
        let decl = f.body.borrow();

        let params = &decl.params;
        let body = &decl.inner;
        let arity = params.len() as u8;

        self.start_function(decl.method, name, arity, 1);

        for p in params {
            self.state_mut().add_local(p.name(), 0);
            self.state_mut().resolve_local(p.name());
        }

        for atom in body.iter() {
            self.compile_atom(atom)
        }

        self.state_mut().end_scope();

        let upvalues = self.state_mut().upvalues.clone();

        let function = self.end_function();
        let handle = self.heap.insert(Object::Function(function)).into_handle();

        let value = Value::object(handle);
        let idx = self.chunk_mut().add_constant(value);

        self.emit(Op::Closure);
        self.emit_byte(idx);
        
        for upvalue in upvalues {
            self.emit_byte(
                if upvalue.is_local {
                    1
                } else {
                    0
                }
            );

            self.emit_byte(upvalue.index)
        }
    }

    fn start_function(&mut self, method: bool, name: &str, arity: u8, scope: usize) {
        let next_function = FunctionBuilder::new(name, arity);
        let reserved_var = if method { "self" } else { "" };
        let state = CompileState::new(method, reserved_var, next_function, scope);

        self.states.push(state)
    }

    fn end_function(&mut self) -> Function {
        self.emit_return(None);

        let mut state = self.states.pop().expect("can't have empty state");

        state.function.set_upvalue_count(state.upvalues.len());
        state.function.build()
    }

    fn resolve_upvalue(&mut self, name: &str) -> u8 {
        let end = self.states.len() - 1;
        let (scope, mut index) =
            self.states[..end].iter_mut()
                .enumerate()
                .rev()
                .filter_map(|(i, enclosing)| {
                    enclosing.capture_local(name).map(|local| (i, local))
                })
                .next()
                .expect("upvalue marked during resolution, but wasn't found");
        
        index = self.states[scope + 1].add_upvalue(index, true);
        
        if scope >= self.states.len() - 2 {
            // if we're one scope from current function
            index
        } else {
            for enclosing in &mut self.states[scope + 2..] {
                index = enclosing.add_upvalue(index, false)
            }

            index
        }
    }

    fn emit_return(&mut self, ret: Option<AtomNode>) {
        let state = self.state_mut();
        let initializer = state.function.name() == "init" && state.method;

        if initializer {
            self.emit(Op::GetLocal);
            self.emit_byte(0)
        } else if let Some(atom) = ret {
            self.compile_atom(atom.inner())
        } else {
            self.emit(Op::Nil)
        }

        self.emit(Op::Return)
    }

    fn state_mut(&mut self) -> &mut CompileState {
        self.states.last_mut().expect("states can't be empty")
    }

    fn chunk_mut(&mut self) -> &mut Chunk {
        self.states.last_mut()
            .expect("states to be non-empty")
            .function
            .chunk_mut()
    }

    fn chunk(&self) -> &Chunk {
        &self.states.last()
            .expect("states to be non-empty")
            .function
            .chunk
    }

    fn line(&mut self) -> usize {
        self.states.last_mut()
            .expect("states to be non-empty")
            .line
    }

    fn string_constant(&mut self, s: &str) -> u8 {
        let chunk = self.states.last_mut().unwrap().function.chunk_mut();

        chunk.string_constant(self.heap, s)
    }

    fn emit(&mut self, op: Op) {
        let line = self.line();
        self.chunk_mut().write(op, line);
    }

    fn emit_byte(&mut self, byte: u8) {
        self.chunk_mut().write_byte(byte);
    }

    fn emit_constant(&mut self, lit: &Literal) {
        use self::Literal::*;

        match *lit {
            Nil     => self.emit(Op::Nil),
            Boolean(b) => self.emit(if b { Op::True} else { Op::False } ),
            Number(n) => self.emit_number_literal(n),
            String(ref s) => {
                let idx = {
                    let chunk = self.states.last_mut().unwrap().function.chunk_mut();
                    chunk.string_constant(self.heap, s)
                };

                self.emit(Op::Constant(idx))
            },

            _ => panic!("not a constant")
        }
    }

    fn emit_number_literal(&mut self, n: f64) {
        self.emit(Op::Immediate);

        let value = Value::float(n).to_raw();
        let chunk = self.chunk_mut();

        chunk.write_u64(value)
    }

    fn emit_jze(&mut self) -> usize {
        let line = self.line();
        let chunk = self.chunk_mut();

        chunk.write(Op::Jump, line);
        chunk.write_byte(0xff);
        chunk.write_byte(0xff);

        chunk.len() - 2
    }

    fn emit_jmp(&mut self) -> usize {
        let line = self.line();
        let chunk = self.chunk_mut();

        chunk.write(Op::Jump, line);
        chunk.write_byte(0xff);
        chunk.write_byte(0xff);
        chunk.len() - 2
    }

    fn emit_loop(&mut self, ip: usize) {
        let line = self.line();
        let chunk = self.chunk_mut();
        let sub = chunk.len() - ip + 3;

        let lo = (sub & 0xff) as u8;
        let hi = ((sub >> 8) & 0xff) as u8;

        chunk.write(Op::Loop, line);
        chunk.write_byte(lo);
        chunk.write_byte(hi);
    }

    fn ip(&self) -> usize {
        self.chunk().len()
    }

    fn patch_jmp(&mut self, idx: usize) {
        let jmp = self.ip();
        let lo = (jmp & 0xff) as u8;
        let hi = ((jmp >> 8) & 0xff) as u8;

        self.chunk_mut().write_byte_at(idx, lo);
        self.chunk_mut().write_byte_at(idx + 1, hi);
    } 
}