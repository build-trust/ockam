use minicbor::{Decode, Encode};
use ockam_abac::{Action, Expr};

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Policy {
    #[n(1)] expression: Expr,
}

impl Policy {
    pub fn new(e: Expr) -> Self {
        Policy { expression: e }
    }

    pub fn expression(&self) -> &Expr {
        &self.expression
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PolicyList {
    #[n(1)] expressions: Vec<Expression>,
}

impl PolicyList {
    pub fn new(e: Vec<Expression>) -> Self {
        PolicyList { expressions: e }
    }

    pub fn expressions(&self) -> &Vec<Expression> {
        &self.expressions
    }
}

#[derive(Debug, Decode, Encode)]
pub struct Expression {
    #[n(1)]
    action: Action,
    #[n(2)]
    expr: Expr,
}

impl Expression {
    pub fn new(action: Action, expr: Expr) -> Self {
        Self { action, expr }
    }

    pub fn action(&self) -> &Action {
        &self.action
    }

    pub fn expr(&self) -> &Expr {
        &self.expr
    }
}
