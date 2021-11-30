// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2021  Philipp Emanuel Weidmann <pew@worldwidemann.com>

use std::collections::HashMap;

use num::{One, ToPrimitive, Zero};

use crate::expression::{Complex, Expression, RationalRepresentation};

/// Error that occurred while trying to evaluate an expression.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Error {
    /// Operation on an expression that the operation is not defined for.
    InvalidOperand {
        expression: Expression,
        operand: Expression,
    },
    /// Operation on two expressions that cannot be combined using the operation.
    IncompatibleOperands {
        expression: Expression,
        operand_1: Expression,
        operand_2: Expression,
    },
    /// Division by an expression that evaluates to zero (undefined).
    DivisionByZero {
        expression: Expression,
        dividend: Expression,
        divisor: Expression,
    },
    /// An expression that evaluates to zero raised to the power of
    /// another expression that evaluates to zero (undefined).
    ZeroToThePowerOfZero {
        expression: Expression,
        base: Expression,
        exponent: Expression,
    },
}

impl Expression {
    /// Returns the result of performing a single evaluation step on
    /// the unary operator expression `self` with operand `a`, or an error
    /// if the expression cannot be evaluated. The `context` argument can be
    /// used to set the values of variables by their identifiers.
    fn evaluate_step_unary(
        &self,
        a: &Self,
        context: &HashMap<String, Self>,
    ) -> Result<Self, Error> {
        use crate::expression::Expression::*;
        use crate::expression::Type::{Arithmetic, Boolean as Bool, Matrix as Mat, Number as Num};
        use Error::*;

        let a_original = a;

        let a = a.evaluate_step(context)?;

        match (self, a.typ()) {
            (Negation(_), Bool(_)) | (Not(_), Num(_, _) | Mat(_) | Arithmetic) => {
                Err(InvalidOperand {
                    expression: self.clone(),
                    operand: a_original.clone(),
                })
            }

            (Negation(_), Num(a, representation)) => Ok(Complex(-a, representation)),
            (Negation(_), Mat(a)) => Ok(Matrix(-a)),
            (Negation(_), _) => Ok(Negation(Box::new(a))),

            (Not(_), Bool(Some(a))) => Ok(Boolean(!a)),
            (Not(_), _) => Ok(Not(Box::new(a))),

            (
                Variable(_)
                | Function(_, _)
                | Integer(_)
                | Rational(_, _)
                | Complex(_, _)
                | Vector(_)
                | Matrix(_)
                | Boolean(_)
                | Sum(_, _)
                | Difference(_, _)
                | Product(_, _)
                | Quotient(_, _)
                | Remainder(_, _)
                | Power(_, _)
                | Equal(_, _)
                | NotEqual(_, _)
                | LessThan(_, _)
                | LessThanOrEqual(_, _)
                | GreaterThan(_, _)
                | GreaterThanOrEqual(_, _)
                | And(_, _)
                | Or(_, _),
                _,
            ) => unreachable!(),
        }
    }

    /// Returns the result of performing a single evaluation step on
    /// the binary operator expression `self` with operands `a` and `b`,
    /// or an error if the expression cannot be evaluated. The `context`
    /// argument can be used to set the values of variables by their
    /// identifiers.
    fn evaluate_step_binary(
        &self,
        a: &Self,
        b: &Self,
        context: &HashMap<String, Self>,
    ) -> Result<Self, Error> {
        use crate::expression::Expression::*;
        use crate::expression::Type::{Arithmetic, Boolean as Bool, Matrix as Mat, Number as Num};
        use Error::*;

        let a_original = a;
        let b_original = b;

        let a = a.evaluate_step(context)?;
        let b = b.evaluate_step(context)?;

        let a_evaluated = &a;
        let b_evaluated = &b;

        match (self, a.typ(), b.typ()) {
            (
                Sum(_, _)
                | Difference(_, _)
                | Product(_, _)
                | Quotient(_, _)
                | Remainder(_, _)
                | Power(_, _),
                Bool(_),
                _,
            )
            | (
                LessThan(_, _)
                | LessThanOrEqual(_, _)
                | GreaterThan(_, _)
                | GreaterThanOrEqual(_, _),
                Mat(_) | Bool(_),
                _,
            )
            | (And(_, _) | Or(_, _), Num(_, _) | Mat(_) | Arithmetic, _) => Err(InvalidOperand {
                expression: self.clone(),
                operand: a_original.clone(),
            }),

            (
                Sum(_, _)
                | Difference(_, _)
                | Product(_, _)
                | Quotient(_, _)
                | Remainder(_, _)
                | Power(_, _),
                _,
                Bool(_),
            )
            | (
                LessThan(_, _)
                | LessThanOrEqual(_, _)
                | GreaterThan(_, _)
                | GreaterThanOrEqual(_, _),
                _,
                Mat(_) | Bool(_),
            )
            | (And(_, _) | Or(_, _), _, Num(_, _) | Mat(_) | Arithmetic) => Err(InvalidOperand {
                expression: self.clone(),
                operand: b_original.clone(),
            }),

            (Sum(_, _) | Difference(_, _) | Equal(_, _) | NotEqual(_, _), Num(_, _), Mat(_))
            | (Sum(_, _) | Difference(_, _) | Equal(_, _) | NotEqual(_, _), Mat(_), Num(_, _))
            | (Equal(_, _) | NotEqual(_, _), Num(_, _) | Mat(_), Bool(_))
            | (Equal(_, _) | NotEqual(_, _), Bool(_), Num(_, _) | Mat(_)) => {
                Err(IncompatibleOperands {
                    expression: self.clone(),
                    operand_1: a_original.clone(),
                    operand_2: b_original.clone(),
                })
            }

            (
                Sum(_, _)
                | Difference(_, _)
                | Product(_, _)
                | Quotient(_, _)
                | Remainder(_, _)
                | Power(_, _)
                | Equal(_, _)
                | NotEqual(_, _)
                | LessThan(_, _)
                | LessThanOrEqual(_, _)
                | GreaterThan(_, _)
                | GreaterThanOrEqual(_, _),
                Num(a, a_representation),
                Num(b, b_representation),
            ) => {
                let representation = a_representation.merge(b_representation);

                match self {
                    Sum(_, _) => Ok(Complex(a + b, representation)),
                    Difference(_, _) => Ok(Complex(a - b, representation)),
                    Product(_, _) => Ok(Complex(a * b, representation)),
                    Quotient(_, _) | Remainder(_, _) => {
                        if b.is_zero() {
                            Err(DivisionByZero {
                                expression: self.clone(),
                                dividend: a_original.clone(),
                                divisor: b_original.clone(),
                            })
                        } else {
                            Ok(Complex(
                                match self {
                                    Quotient(_, _) => a / b,
                                    Remainder(_, _) => a % b,
                                    _ => unreachable!(),
                                },
                                representation,
                            ))
                        }
                    }
                    Power(_, _) => {
                        if a.is_zero() && b.is_zero() {
                            Err(ZeroToThePowerOfZero {
                                expression: self.clone(),
                                base: a_original.clone(),
                                exponent: b_original.clone(),
                            })
                        } else if let Some(b) = b.to_i32() {
                            Ok(Complex(a.powi(b), representation))
                        } else {
                            // TODO
                            Ok(Power(
                                Box::new(a_evaluated.clone()),
                                Box::new(b_evaluated.clone()),
                            ))
                        }
                    }
                    Equal(_, _) => Ok(Boolean(a == b)),
                    NotEqual(_, _) => Ok(Boolean(a != b)),
                    LessThan(_, _)
                    | LessThanOrEqual(_, _)
                    | GreaterThan(_, _)
                    | GreaterThanOrEqual(_, _) => {
                        if !a.im.is_zero() {
                            Err(InvalidOperand {
                                expression: self.clone(),
                                operand: a_original.clone(),
                            })
                        } else if !b.im.is_zero() {
                            Err(InvalidOperand {
                                expression: self.clone(),
                                operand: b_original.clone(),
                            })
                        } else {
                            let a = a.re;
                            let b = b.re;

                            Ok(Boolean(match self {
                                LessThan(_, _) => a < b,
                                LessThanOrEqual(_, _) => a <= b,
                                GreaterThan(_, _) => a > b,
                                GreaterThanOrEqual(_, _) => a >= b,
                                _ => unreachable!(),
                            }))
                        }
                    }
                    _ => unreachable!(),
                }
            }

            (Equal(_, _), Bool(Some(a)), Bool(Some(b))) => Ok(Boolean(a == b)),
            (NotEqual(_, _), Bool(Some(a)), Bool(Some(b))) => Ok(Boolean(a != b)),
            (And(_, _), Bool(Some(a)), Bool(Some(b))) => Ok(Boolean(a && b)),
            (Or(_, _), Bool(Some(a)), Bool(Some(b))) => Ok(Boolean(a || b)),

            (Sum(_, _), _, _) => Ok(Sum(Box::new(a), Box::new(b))), // TODO
            (Difference(_, _), _, _) => Ok(Difference(Box::new(a), Box::new(b))), // TODO
            (Product(_, _), _, _) => Ok(Product(Box::new(a), Box::new(b))), // TODO
            (Quotient(_, _), _, _) => Ok(Quotient(Box::new(a), Box::new(b))), // TODO
            (Remainder(_, _), _, _) => Ok(Remainder(Box::new(a), Box::new(b))), // TODO
            (Power(_, _), _, _) => Ok(Power(Box::new(a), Box::new(b))), // TODO
            (Equal(_, _), _, _) => Ok(Equal(Box::new(a), Box::new(b))), // TODO
            (NotEqual(_, _), _, _) => Ok(NotEqual(Box::new(a), Box::new(b))), // TODO
            (LessThan(_, _), _, _) => Ok(LessThan(Box::new(a), Box::new(b))), // TODO
            (LessThanOrEqual(_, _), _, _) => Ok(LessThanOrEqual(Box::new(a), Box::new(b))), // TODO
            (GreaterThan(_, _), _, _) => Ok(GreaterThan(Box::new(a), Box::new(b))), // TODO
            (GreaterThanOrEqual(_, _), _, _) => Ok(GreaterThanOrEqual(Box::new(a), Box::new(b))), // TODO
            (And(_, _), _, _) => Ok(And(Box::new(a), Box::new(b))), // TODO
            (Or(_, _), _, _) => Ok(Or(Box::new(a), Box::new(b))),   // TODO

            (
                Variable(_)
                | Function(_, _)
                | Integer(_)
                | Rational(_, _)
                | Complex(_, _)
                | Vector(_)
                | Matrix(_)
                | Boolean(_)
                | Negation(_)
                | Not(_),
                _,
                _,
            ) => unreachable!(),
        }
    }

    /// Returns the result of performing a single evaluation step on the expression,
    /// or an error if the expression cannot be evaluated. The `context` argument
    /// can be used to set the values of variables by their identifiers.
    fn evaluate_step(&self, context: &HashMap<String, Self>) -> Result<Self, Error> {
        use crate::expression::Expression::*;

        match self {
            Variable(identifier) => context
                .get(identifier)
                .map_or_else(|| Ok(self.clone()), |x| x.evaluate_step(context)),
            Function(_, _) => Ok(self.clone()), // TODO
            Integer(_) => Ok(self.clone()),
            Rational(x, _) => Ok(if x.denom().is_one() {
                Integer(x.numer().clone())
            } else {
                self.clone()
            }),
            Complex(z, representation) => Ok(if z.im.is_zero() {
                Rational(z.re.clone(), *representation)
            } else {
                self.clone()
            }),
            Vector(_) => Ok(self.clone()), // TODO: Evaluate each element!
            Matrix(_) => Ok(self.clone()), // TODO: Evaluate each element!
            Boolean(_) => Ok(self.clone()),
            Negation(a) => self.evaluate_step_unary(a, context),
            Not(a) => self.evaluate_step_unary(a, context),
            Sum(a, b) => self.evaluate_step_binary(a, b, context),
            Difference(a, b) => self.evaluate_step_binary(a, b, context),
            Product(a, b) => self.evaluate_step_binary(a, b, context),
            Quotient(a, b) => self.evaluate_step_binary(a, b, context),
            Remainder(a, b) => self.evaluate_step_binary(a, b, context),
            Power(a, b) => self.evaluate_step_binary(a, b, context),
            Equal(a, b) => self.evaluate_step_binary(a, b, context),
            NotEqual(a, b) => self.evaluate_step_binary(a, b, context),
            LessThan(a, b) => self.evaluate_step_binary(a, b, context),
            LessThanOrEqual(a, b) => self.evaluate_step_binary(a, b, context),
            GreaterThan(a, b) => self.evaluate_step_binary(a, b, context),
            GreaterThanOrEqual(a, b) => self.evaluate_step_binary(a, b, context),
            And(a, b) => self.evaluate_step_binary(a, b, context),
            Or(a, b) => self.evaluate_step_binary(a, b, context),
        }
    }

    /// Returns the result of evaluating the expression, or an error
    /// if the expression cannot be evaluated. The `context` argument
    /// can be used to set the values of variables by their identifiers.
    pub fn evaluate(&self, context: HashMap<String, Self>) -> Result<Self, Error> {
        let mut default_context = HashMap::new();

        default_context.insert(
            "i".to_owned(),
            Expression::Complex(Complex::i(), RationalRepresentation::Fraction),
        );

        for (identifier, expression) in context {
            default_context.insert(identifier, expression);
        }

        let mut old_expression = self.clone();

        loop {
            let new_expression = old_expression.evaluate_step(&default_context)?;

            if new_expression == old_expression {
                return Ok(new_expression);
            }

            old_expression = new_expression;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::expression::Expression;

    #[track_caller]
    fn t(expression: &str, result: &str) {
        assert_eq!(
            expression
                .parse::<Expression>()
                .unwrap()
                .evaluate(HashMap::new())
                .unwrap()
                .to_string(),
            result,
        );
    }

    #[test]
    fn arithmetic() {
        t("-(-1)", "1");
        t("-0", "0");

        t("1 + 2", "3");
        t("1 + -1", "0");
        t("1/2 + 0.5", "1");
        t(
            "123456789987654321 + 987654321123456789",
            "1111111111111111110",
        );

        t("1 - 2", "-1");
        t("1 - -1", "2");
        t("1/2 - 0.5", "0");
        t(
            "123456789987654321 - 987654321123456789",
            "-864197531135802468",
        );

        t("1 * 2", "2");
        t("1 * -1", "-1");
        t("1/2 * 0.5", "0.25");
        t(
            "123456789987654321 * 987654321123456789",
            "121932632103337905662094193112635269",
        );

        t("1 / 2", "1/2");
        t("1 / -1", "-1");
        t("1/2 / 0.5", "1");
        t(
            "123456789987654321 / 987654321123456789",
            "101010101/808080809",
        );

        t("4 % 2", "0");
        t("0 % 2", "0");
        t("5 % 2", "1");
        t("-5 % 2", "-1");
        t("-5 % -2", "-1");
        t("0.75 % (1/4)", "0");
        t("0.75 % (1/3)", "1/12");
        t("987654321123456789 % 123456789987654321", "1222222221");

        t("i ^ 2", "-1");
        t("2 ^ 3", "8");
        t("2 ^ (-3)", "1/8");
        t("-2 ^ 4", "-16");
        t("(-2) ^ 4", "16");
        t("0.5 ^ 4", "0.0625");
        t("987654321123456789 ^ 5", "939777062588963894467852986656442266299580252508947542802086985660852317355013741720482949");
        t("3 ^ 4 ^ 5", "373391848741020043532959754184866588225409776783734007750636931722079040617265251229993688938803977220468765065431475158108727054592160858581351336982809187314191748594262580938807019951956404285571818041046681288797402925517668012340617298396574731619152386723046235125934896058590588284654793540505936202376547807442730582144527058988756251452817793413352141920744623027518729185432862375737063985485319476416926263819972887006907013899256524297198527698749274196276811060702333710356481");
    }

    #[test]
    fn logic() {
        t("!true", "false");
        t("!false", "true");

        t("true && true", "true");
        t("true && false", "false");
        t("false && true", "false");
        t("false && false", "false");

        t("true || true", "true");
        t("true || false", "true");
        t("false || true", "true");
        t("false || false", "false");
    }

    #[test]
    fn comparisons() {
        t("0 == 0", "true");
        t("0 == 0.0", "true");
        t("0.5 == 1/2", "true");
        t("1/2 == 2/4", "true");
        t("3 ^ 4 ^ 5 == 5 ^ 4 ^ 3", "false");

        t("0 != 0", "false");
        t("0 != 0.0", "false");
        t("0.5 != 1/2", "false");
        t("1/2 != 2/4", "false");
        t("3 ^ 4 ^ 5 != 5 ^ 4 ^ 3", "true");

        t("0 < 0", "false");
        t("0 < 0.0", "false");
        t("0.5 < 1/2", "false");
        t("1/2 < 2/4", "false");
        t("3 ^ 4 ^ 5 < 5 ^ 4 ^ 3", "false");

        t("0 <= 0", "true");
        t("0 <= 0.0", "true");
        t("0.5 <= 1/2", "true");
        t("1/2 <= 2/4", "true");
        t("3 ^ 4 ^ 5 <= 5 ^ 4 ^ 3", "false");

        t("0 > 0", "false");
        t("0 > 0.0", "false");
        t("0.5 > 1/2", "false");
        t("1/2 > 2/4", "false");
        t("3 ^ 4 ^ 5 > 5 ^ 4 ^ 3", "true");

        t("0 >= 0", "true");
        t("0 >= 0.0", "true");
        t("0.5 >= 1/2", "true");
        t("1/2 >= 2/4", "true");
        t("3 ^ 4 ^ 5 >= 5 ^ 4 ^ 3", "true");

        t("true == true", "true");
        t("true == false", "false");
        t("false == true", "false");
        t("false == false", "true");

        t("true != true", "false");
        t("true != false", "true");
        t("false != true", "true");
        t("false != false", "false");
    }
}