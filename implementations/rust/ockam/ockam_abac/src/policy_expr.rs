use crate::policy_expr::PolicyExpression::{BooleanExpression, FullExpression};
use crate::{BooleanExpr, Expr};
#[cfg(feature = "std")]
use core::str::FromStr;
use minicbor::{CborLen, Decode, Encode};
#[cfg(feature = "std")]
use ockam_core::compat::fmt::{Display, Formatter};
#[cfg(feature = "std")]
use ockam_core::Result;

/// A Policy expression can either be represented with
///   - A full expression with string valued attributes, contain operator, etc...
///   - A simpler boolean expression with just and / or / not operators acting on boolean attributes
#[derive(Debug, Clone, Encode, Decode, CborLen)]
pub enum PolicyExpression {
    #[n(0)]
    FullExpression(#[n(0)] Expr),
    #[n(1)]
    BooleanExpression(#[n(0)] BooleanExpr),
}

impl From<PolicyExpression> for Expr {
    fn from(value: PolicyExpression) -> Self {
        value.to_expression()
    }
}

impl PolicyExpression {
    /// Return the policy expression corresponding to either a fully expanded policy expression
    /// or a boolean expression
    pub fn to_expression(&self) -> Expr {
        match self {
            FullExpression(e) => e.clone(),
            BooleanExpression(e) => e.to_expression(),
        }
    }
}

impl PartialEq for PolicyExpression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FullExpression(e1), FullExpression(e2)) => e1 == e2,
            (BooleanExpression(e1), BooleanExpression(e2)) => e1 == e2,
            _ => false,
        }
    }
}

impl Eq for PolicyExpression {}

#[cfg(feature = "std")]
impl Display for PolicyExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            FullExpression(e) => e.fmt(f),
            BooleanExpression(e) => e.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl TryFrom<&str> for PolicyExpression {
    type Error = crate::ParseError;

    /// Try to parse the expression first as a simple boolean expression,
    /// then as a full policy expression
    fn try_from(input: &str) -> Result<Self, Self::Error> {
        match BooleanExpr::try_from(input) {
            Ok(expression) => Ok(BooleanExpression(expression)),
            Err(e1) => match Expr::try_from(input) {
                Ok(expression) => Ok(FullExpression(expression)),
                Err(e2) => Err(crate::ParseError::message(format!("Cannot parse the expression as either a simple boolean expression or a full policy expression:\n - {e1}\n - {e2}")))
            }
        }
    }
}

#[cfg(feature = "std")]
impl FromStr for PolicyExpression {
    type Err = crate::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

#[cfg(feature = "std")]
impl TryFrom<String> for PolicyExpression {
    type Error = crate::ParseError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Self::try_from(input.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::PolicyExpression::{BooleanExpression, FullExpression};
    use crate::{BooleanExpr, Expr, PolicyExpression};
    use core::str::FromStr;

    #[test]
    fn from_str() {
        let full_expression = "(= subject.test = \"true\")";
        assert_eq!(
            PolicyExpression::from_str(full_expression).unwrap(),
            FullExpression(Expr::from_str(full_expression).unwrap())
        );

        let boolean_expression = "test";
        assert_eq!(
            PolicyExpression::from_str(boolean_expression).unwrap(),
            BooleanExpression(BooleanExpr::from_str(boolean_expression).unwrap())
        );
    }
}
