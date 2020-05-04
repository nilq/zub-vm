use super::*;

use std::rc::Rc;
use std::cell::RefCell;

pub struct IrBuilder {
    program: Vec<ExprNode>,
}

impl IrBuilder {
    pub fn new() -> Self {
        IrBuilder {
            program: Vec::new(),
        }
    }


    pub fn bind(&mut self, binding: Binding, rhs: ExprNode) {
        let bind = Expr::Bind(binding, rhs);

        self.emit(bind.node(TypeInfo::nil()));
    }

    pub fn mutate(&mut self, lhs: ExprNode, rhs: ExprNode) {
        let mutate = Expr::Mutate(lhs, rhs);

        self.emit(mutate.clone().node(TypeInfo::nil()))
    }

    pub fn ret(&mut self, value: Option<ExprNode>) {
        let info = if let Some(ref value) = value {
            value.type_info().clone()
        } else {
            TypeInfo::nil()
        };

        self.emit(
            Expr::Return(value).node(info)
        )
    }



    pub fn list(&self, content: Vec<ExprNode>) -> ExprNode {
        Expr::List(content).node(TypeInfo::nil())
    }

    pub fn list_get(&self, list: ExprNode, index: ExprNode) -> ExprNode {
        Expr::ListGet(list, index).node(TypeInfo::nil())
    }

    pub fn list_set(&self, list: ExprNode, index: ExprNode, value: ExprNode) -> ExprNode {
        Expr::ListSet(list, index, value).node(TypeInfo::nil())
    }



    pub fn var(&self, binding: Binding) -> ExprNode {
        Expr::Var(
            binding
        ).node(
            TypeInfo::nil()
        )
    }

    pub fn call(&mut self, callee: ExprNode, args: Vec<ExprNode>, retty: Option<TypeInfo>) -> ExprNode {
        let call = Call {
            callee,
            args
        };

        Expr::Call(call).node(
            if let Some(info) = retty {
                info
            } else {
                TypeInfo::nil()
            }
        )
    }



    pub fn binary(&self, lhs: ExprNode, op: BinaryOp, rhs: ExprNode) -> ExprNode {
        Expr::Binary(lhs, op, rhs).node(TypeInfo::nil())
    }

    pub fn unary(op: UnaryOp, rhs: ExprNode) -> Expr {
        Expr::Unary(op, rhs)
    }

    pub fn int(&mut self, n: i32) -> ExprNode {
        let info = TypeInfo::new(Type::Int);
        let lit = Literal::Number(n as f64);

        Expr::Literal(lit).node(info)
    }

    pub fn number(&mut self, n: f64) -> ExprNode {
        let info = TypeInfo::new(Type::Float);
        let lit = Literal::Number(n);

        Expr::Literal(lit).node(info)
    }

    pub fn string(&mut self, s: &str) -> ExprNode {
        let info = TypeInfo::new(Type::String);
        let lit = Literal::String(s.to_owned());

        Expr::Literal(lit).node(info)
    }

    pub fn bool(&mut self, b: bool) -> ExprNode {
        let info = TypeInfo::new(Type::Bool);
        let lit = Literal::Boolean(b);

        Expr::Literal(lit).node(info)
    }



    pub fn function(&mut self, var: Binding, params: &[&str], mut body_build: impl FnMut(&mut IrBuilder)) -> ExprNode {
        let mut body_builder = IrBuilder::new();

        body_build(&mut body_builder);

        let body = body_builder.build();

        let func_body = IrFunctionBody {
            params: params.iter().cloned().map(|x: &str|
                Binding::local(x, var.depth.unwrap_or(0) + 1, var.function_depth + 1)).collect::<Vec<Binding>>(),
            method: false,
            inner: body
        };

        let ir_func = IrFunction {
            var,
            body: Rc::new(RefCell::new(func_body))
        };

        Expr::Function(
            ir_func
        ).node(
            TypeInfo::nil()
        )
    }

    pub fn ternary(&mut self, cond: ExprNode, then_body: ExprNode, else_body: Option<ExprNode>) -> ExprNode {
        Expr::If(
            cond,
            then_body,
            else_body
        ).node(TypeInfo::nil())
    }

    pub fn if_(&mut self, cond: ExprNode, then_build: fn(&mut IrBuilder), else_build: Option<fn(&mut IrBuilder)>) -> ExprNode {
        let mut then_builder = IrBuilder::new();

        then_build(&mut then_builder);

        let then_body = Expr::Block(then_builder.build()).node(TypeInfo::nil());

        let else_body = if let Some(else_build) = else_build {
            let mut else_builder = IrBuilder::new();

            else_build(&mut else_builder);

            Some(Expr::Block(else_builder.build()).node(TypeInfo::nil()))
        } else {
            None
        };

        Expr::If(
            cond,
            then_body,
            else_body
        ).node(TypeInfo::nil())
    }

    pub fn while_(&mut self, cond: ExprNode, then_build: fn(&mut IrBuilder)) -> ExprNode {
        let mut then_builder = IrBuilder::new();

        then_build(&mut then_builder);

        let then_body = Expr::Block(then_builder.build()).node(TypeInfo::nil());

        Expr::While(
            cond,
            then_body,
        ).node(TypeInfo::nil())
    }



    pub fn build(self) -> Vec<ExprNode> {
        self.program
    }

    pub fn emit(&mut self, atom: ExprNode) {
        self.program.push(atom)
    }
}
