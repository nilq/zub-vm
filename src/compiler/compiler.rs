use super::chunk::{ Chunk, Op };
use super::*;

#[derive(Debug, Clone)]
pub struct Local {
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
pub struct CompileState {
    line: usize,
    pub locals: Vec<Local>,
    upvalues: Vec<UpValue>,
    function: FunctionBuilder,
    scope_depth: usize,
    breaks: Vec<usize>,
    method: bool,
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
        let depth = self.scope_depth - (depth-1);

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

        panic!("TODO: unresolved var: {} in {:#?}", var, self.locals)
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
    pub states: Vec<CompileState>,
    pub locals_cache: Vec<Local>,
}

impl<'g> Compiler<'g> {
    pub fn new(heap: &'g mut Heap<Object>) -> Self {
        Compiler {
            heap,
            states: Vec::new(),
            locals_cache: Vec::new(),
        }
    }

    pub fn compile(&mut self, exprs: &[ExprNode]) -> Function {
        self.start_function(false, "<zub>", 0, 0);

        for expr in exprs.iter() {
            self.compile_expr(expr)
        }

        self.emit_return(None);
        self.end_function()
    }

    pub fn compile_from(&mut self, exprs: &[ExprNode], locals: Vec<Local>) -> Function {
        self.start_function(false, "<zub>", 0, 0);
        self.states.last_mut().unwrap().locals = locals;

        for expr in exprs.iter() {
            self.compile_expr(expr)
        }

        self.emit_return(None);
        self.end_function()
    }

    fn compile_expr(&mut self, expr: &ExprNode) {
        use self::Expr::*;

        match expr.inner() {
            Literal(ref lit) => self.emit_constant(lit),
            Unary(ref op, ref node) => {
                self.compile_expr(node);

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
                    self.compile_expr(rhs);

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

            Function(ref ir_func) => {
                self.function_decl(ir_func);
                self.var_define(&ir_func.var, None);
            },

            AnonFunction(ref ir_func) => {
                self.function_decl(ir_func);
            }

            Not(ref expr) => {
                self.compile_expr(expr);
                self.emit(Op::Not)
            }

            Neg(ref expr) => {
                self.compile_expr(expr);
                self.emit(Op::Neg)
            }

            Call(ref call) => {
                let arity = call.args.len();

                if arity > 8 {
                    panic!("That's a lot of arguments. But I will fix this limitation asap.")
                }

                self.compile_expr(&call.callee);

                for arg in call.args.iter() {
                    self.compile_expr(arg)
                }

                self.emit(Op::Call(arity as u8))
            },

            List(ref content) => {
                for el in content.iter().rev() {
                    self.compile_expr(el)
                }

                self.emit(Op::List);
                self.emit_byte(content.len() as u8)
            },

            SetElement(ref list, ref index, ref value) => {
                self.compile_expr(value);
                self.compile_expr(index);
                self.compile_expr(list);

                self.emit(Op::SetElement);
            },

            Dict(keys, values) => {
                for (key, val) in keys.iter().zip(values.iter()) {
                    self.compile_expr(key);
                    self.compile_expr(val);
                }

                self.emit(Op::Dict);
                self.emit_byte(keys.len() as u8);
            },

            If(ref cond, ref then, ref els) => {
                self.compile_expr(cond);

                let else_jmp = self.emit_jze();

                self.emit(Op::Pop);
                self.compile_expr(then);

                let end_jmp = self.emit_jmp();

                self.patch_jmp(else_jmp);
                self.emit(Op::Pop);

                if let &Some(ref els) = els {
                    self.compile_expr(els)
                }

                self.patch_jmp(end_jmp)
            },

            While(ref cond, ref body) => {
                let ip = self.ip();

                self.compile_expr(cond);

                let end_jmp = self.emit_jze();

                self.emit(Op::Pop);
                self.compile_expr(body);

                self.emit_loop(ip);
                self.patch_jmp(end_jmp);

                self.emit(Op::Pop);

                for b in self.state_mut().breaks() {
                    self.patch_jmp(b)
                }
            },

            Break => {
                let jmp = self.emit_jmp();
                self.state_mut().add_break(jmp)
            },

            Pop => {
                self.emit(Op::Pop)
            }

            Binary(lhs, op, rhs) => {
                use self::BinaryOp::*;

                match op {
                    And => {
                        self.compile_expr(lhs);

                        let short_circuit_jmp = self.emit_jze();

                        self.emit(Op::Pop);
                        self.compile_expr(rhs);

                        self.patch_jmp(short_circuit_jmp);
                    },

                    Or => {
                        self.compile_expr(lhs);

                        let else_jmp = self.emit_jze();
                        let end_jmp = self.emit_jmp();

                        self.patch_jmp(else_jmp);
                        self.emit(Op::Pop);

                        self.compile_expr(rhs);

                        self.patch_jmp(end_jmp)
                    },

                    Index => {
                        self.compile_expr(rhs);
                        self.compile_expr(lhs);
        
                        self.emit(Op::Index);
                    }

                    _ => {
                        // This looks kinda funny, but it's an ok way of matching I guess

                        self.compile_expr(lhs); // will handle type in the future :)
                        self.compile_expr(rhs);

                        match op {
                            Add => self.emit(Op::Add),
                            Sub => self.emit(Op::Sub),
                            Rem => self.emit(Op::Rem),
                            Mul => self.emit(Op::Mul),
                            Div => self.emit(Op::Div),

                            Equal => self.emit(Op::Equal),
                            Gt => self.emit(Op::Greater),
                            Lt => self.emit(Op::Less),
                            Pow => self.emit(Op::Pow),

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
                self.compile_expr(init);
                self.var_define(var, None);
            },

            BindGlobal(ref var, ref init) => {
                self.compile_expr(init);
                self.var_define(var, None)
            },

            Block(ref body) => for node in body {
                self.compile_expr(node)
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
            self.state_mut().add_local(p.name(), 1);
            self.state_mut().resolve_local(p.name());
        }

        for expr in body.iter() {
            self.compile_expr(expr)
        }

        self.state_mut().end_scope();

        let upvalues = self.state_mut().upvalues.clone();

        let function = self.end_function(); // Might delete later, felt cute
        let handle = self.heap.insert(Object::Function(function)).into_handle();

        let value = Value::object(handle);

        self.emit(Op::Closure);

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
        
        let idx = self.chunk_mut().add_constant(value);
        self.emit_byte(idx);
    }

    fn start_function(&mut self, method: bool, name: &str, arity: u8, scope: usize) {
        let next_function = FunctionBuilder::new(name, arity);
        let reserved_var = if method { "self" } else { "" };
        let state = CompileState::new(method, reserved_var, next_function, scope);

        self.states.push(state)
    }

    fn end_function(&mut self) -> Function {
        // self.emit_return(None);

        let mut state: CompileState = self.states.pop().expect("states can't be empty");

        self.locals_cache.extend(state.locals.clone());

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
                .expect(&format!("upvalue marked during resolution, but wasn't found: {}", name));


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

    fn emit_return(&mut self, ret: Option<ExprNode>) {
        let state = self.state_mut();
        let initializer = state.function.name() == "init" && state.method;

        if initializer {
            self.emit(Op::GetLocal);
            self.emit_byte(0)
        } else if let Some(ref expr) = ret {
            self.compile_expr(expr)
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

        chunk.write(Op::JumpIfFalse, line);
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

    /// Patches jump instruction to jump to current
    /// Instruction Pointer (IP)
    fn patch_jmp(&mut self, idx: usize) {
        let jmp = self.ip();
        let lo = (jmp & 0xff) as u8;
        let hi = ((jmp >> 8) & 0xff) as u8;

        self.chunk_mut().write_byte_at(idx, lo);
        self.chunk_mut().write_byte_at(idx + 1, hi);
    }
}
