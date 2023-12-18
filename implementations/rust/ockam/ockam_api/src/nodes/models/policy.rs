use minicbor::{Decode, Encode};
use ockam_abac::{Action, Expr};

#[derive(Debug, Clone, Decode, Encode)]
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

#[derive(Debug, Clone, Decode, Encode)]
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

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    use ockam_abac::{Action, Expr};

    use crate::nodes::models::policy::{Expression, PolicyList};
    use crate::schema::tests::validate_with_schema;

    quickcheck! {
        fn policy_list(g: PolicyList) -> TestResult {
            validate_with_schema("policy_list", g)
        }
    }

    quickcheck! {
        fn policy(g: Expression) -> TestResult {
            validate_with_schema("expression", g)
        }
    }

    impl Arbitrary for PolicyList {
        fn arbitrary(g: &mut Gen) -> Self {
            PolicyList {
                expressions: vec![
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::Str(String::arbitrary(g)),
                    },
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::Int(i64::arbitrary(g)),
                    },
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::Float(f64::arbitrary(g)),
                    },
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::Bool(bool::arbitrary(g)),
                    },
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::Ident(String::arbitrary(g)),
                    },
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::Seq(vec![
                            Expr::Str(String::arbitrary(g)),
                            Expr::Str(String::arbitrary(g)),
                        ]),
                    },
                    Expression {
                        action: Action::new(String::arbitrary(g).as_str()),
                        expr: Expr::List(vec![
                            Expr::Int(i64::arbitrary(g)),
                            Expr::Float(f64::arbitrary(g)),
                        ]),
                    },
                ],
            }
        }
    }

    impl Arbitrary for Expression {
        fn arbitrary(g: &mut Gen) -> Self {
            Expression {
                action: Action::new(String::arbitrary(g).as_str()),
                expr: Expr::List(vec![
                    Expr::Int(i64::arbitrary(g)),
                    Expr::Float(f64::arbitrary(g)),
                ]),
            }
        }
    }
}
