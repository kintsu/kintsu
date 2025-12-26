//! Type Expression AST nodes
//!
//! Defines compile-time type transformation operators per RFC-0018 and SPEC-0017.
//! Type expressions derive new types from existing types through operators like
//! Pick, Omit, Partial, Required, Exclude, Extract, and ArrayItem.
//!
//! **Spec references:** RFC-0018, SPEC-0017, TSY-0014

use serde::{Deserialize, Serialize};

use crate::{
    SpannedToken, Token,
    defs::Spanned,
    fmt::Printer,
    tokens::{AstResult, LexingError, Parse, Peek, ToTokens, TokenStream, bracket},
};

use super::ty::PathOrIdent;

/// Selector list for field-based operations (Pick, Omit, Partial, Required).
/// Contains pipe-separated identifiers: `field1 | field2 | field3`
#[derive(Clone, Serialize, Deserialize)]
pub struct SelectorList {
    /// Field names selected
    pub fields: Vec<Spanned<SpannedToken![ident]>>,
}

impl Parse for SelectorList {
    fn parse(stream: &mut TokenStream) -> AstResult<Self> {
        let mut fields = vec![];

        // Parse first field (required)
        let first: Spanned<SpannedToken![ident]> = stream.parse()?;
        fields.push(first);

        // Parse additional fields separated by |
        while stream.peek::<Token![|]>() {
            let _pipe: SpannedToken![|] = stream.parse()?;
            let field: Spanned<SpannedToken![ident]> = stream.parse()?;
            fields.push(field);
        }

        Ok(Self { fields })
    }
}

/// Variant list for oneof-based operations (Exclude, Extract).
/// Contains pipe-separated type identifiers: `Variant1 | Variant2`
#[derive(Clone, Serialize, Deserialize)]
pub struct VariantList {
    /// Variant names selected
    pub variants: Vec<Spanned<SpannedToken![ident]>>,
}

impl Parse for VariantList {
    fn parse(stream: &mut TokenStream) -> AstResult<Self> {
        let mut variants = vec![];

        // Parse first variant (required)
        let first: Spanned<SpannedToken![ident]> = stream.parse()?;
        variants.push(first);

        // Parse additional variants separated by |
        while stream.peek::<Token![|]>() {
            let _pipe: SpannedToken![|] = stream.parse()?;
            let variant: Spanned<SpannedToken![ident]> = stream.parse()?;
            variants.push(variant);
        }

        Ok(Self { variants })
    }
}

/// Type expression operator kinds per SPEC-0017.
///
/// These operators transform types at compile time:
/// - Struct operators: Pick, Omit, Partial, Required
/// - OneOf operators: Exclude, Extract
/// - Array operators: ArrayItem
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeExprOp {
    /// Pick specific fields: `Pick[User, id | name]`
    Pick {
        target: Box<TypeExpr>,
        fields: SelectorList,
    },
    /// Omit specific fields: `Omit[User, password]`
    Omit {
        target: Box<TypeExpr>,
        fields: SelectorList,
    },
    /// Make fields optional: `Partial[User]` or `Partial[User, name | email]`
    Partial {
        target: Box<TypeExpr>,
        fields: Option<SelectorList>,
    },
    /// Make fields required: `Required[User]` or `Required[User, id]`
    Required {
        target: Box<TypeExpr>,
        fields: Option<SelectorList>,
    },
    /// Exclude variants from oneof: `Exclude[Status, Pending | Failed]`
    Exclude {
        target: Box<TypeExpr>,
        variants: VariantList,
    },
    /// Extract variants from oneof: `Extract[Status, Success]`
    Extract {
        target: Box<TypeExpr>,
        variants: VariantList,
    },
    /// Get array element type: `ArrayItem[Users]`
    ArrayItem { target: Box<TypeExpr> },
}

/// Check if an identifier is a type expression operator keyword
fn is_type_expr_op(name: &str) -> bool {
    matches!(
        name,
        "Pick" | "Omit" | "Partial" | "Required" | "Exclude" | "Extract" | "ArrayItem"
    )
}

/// Peek to check if we're at a type expression operator
fn peek_type_expr_op(stream: &TokenStream) -> bool {
    let mut fork = stream.fork();
    if let Ok(ident) = <SpannedToken![ident]>::parse(&mut fork) {
        let name = ident.borrow_string();
        is_type_expr_op(&name) && fork.peek::<crate::tokens::LBracketToken>()
    } else {
        false
    }
}

impl Parse for TypeExprOp {
    fn parse(stream: &mut TokenStream) -> AstResult<Self> {
        // Parse operator name
        let op_name: SpannedToken![ident] = stream.parse()?;
        let op_str = op_name.borrow_string().clone();

        // Parse bracket content
        let mut bracketed;
        let _bracket = bracket!(bracketed in stream);

        // Parse target type expression
        let target = Box::new(TypeExpr::parse(&mut bracketed)?);

        match op_str.as_str() {
            "Pick" => {
                let _: SpannedToken![,] = bracketed.parse()?;
                let fields = SelectorList::parse(&mut bracketed)?;
                Ok(Self::Pick { target, fields })
            },
            "Omit" => {
                let _: SpannedToken![,] = bracketed.parse()?;
                let fields = SelectorList::parse(&mut bracketed)?;
                Ok(Self::Omit { target, fields })
            },
            "Partial" => {
                let fields = if bracketed.peek::<Token![,]>() {
                    let _: SpannedToken![,] = bracketed.parse()?;
                    Some(SelectorList::parse(&mut bracketed)?)
                } else {
                    None
                };
                Ok(Self::Partial { target, fields })
            },
            "Required" => {
                let fields = if bracketed.peek::<Token![,]>() {
                    let _: SpannedToken![,] = bracketed.parse()?;
                    Some(SelectorList::parse(&mut bracketed)?)
                } else {
                    None
                };
                Ok(Self::Required { target, fields })
            },
            "Exclude" => {
                let _: SpannedToken![,] = bracketed.parse()?;
                let variants = VariantList::parse(&mut bracketed)?;
                Ok(Self::Exclude { target, variants })
            },
            "Extract" => {
                let _: SpannedToken![,] = bracketed.parse()?;
                let variants = VariantList::parse(&mut bracketed)?;
                Ok(Self::Extract { target, variants })
            },
            "ArrayItem" => Ok(Self::ArrayItem { target }),
            _ => {
                Err(LexingError::unknown_type_expr_op(
                    vec![
                        "Pick",
                        "Omit",
                        "Partial",
                        "Required",
                        "Exclude",
                        "Extract",
                        "ArrayItem",
                    ],
                    op_str,
                    &op_name.span,
                ))
            },
        }
    }
}

impl Peek for TypeExprOp {
    fn peek(stream: &TokenStream) -> bool {
        peek_type_expr_op(stream)
    }
}

/// Type expression AST node per SPEC-0017 Section 4.
///
/// Type expressions support:
/// - Type references (base case)
/// - Field access via `::` operator
/// - Transformation operators (Pick, Omit, etc.)
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TypeExpr {
    /// Reference to a named type: `User` or `geometry::Point`
    TypeRef { reference: PathOrIdent },
    /// Field access: `User::profile::avatar`
    /// The `::` operator binds tighter than binary type operators
    FieldAccess {
        base: Box<Spanned<TypeExpr>>,
        /// The `::` token
        sep: SpannedToken![::],
        field: SpannedToken![ident],
    },
    /// Type expression operator application
    Op(Spanned<TypeExprOp>),
}

impl TypeExpr {
    /// Returns true if this is a simple type reference
    pub fn is_type_ref(&self) -> bool {
        matches!(self, Self::TypeRef { .. })
    }

    /// Returns true if this is a field access expression
    pub fn is_field_access(&self) -> bool {
        matches!(self, Self::FieldAccess { .. })
    }

    /// Returns true if this is an operator expression
    pub fn is_op(&self) -> bool {
        matches!(self, Self::Op(_))
    }
}

impl Parse for TypeExpr {
    fn parse(stream: &mut TokenStream) -> AstResult<Self> {
        let start = stream.cursor();

        // Check for type expression operator (Pick, Omit, etc.)
        if TypeExprOp::peek(stream) {
            let op = TypeExprOp::parse(stream)?;
            let end = stream.cursor();
            return Ok(Self::Op(Spanned::new(start, end, op)));
        }

        // Parse base type reference
        let reference = PathOrIdent::parse(stream)?;
        let mut result = Self::TypeRef { reference };

        // Handle field access chain (User::profile::avatar)
        while stream.peek::<Token![::]>() {
            let end = stream.cursor();
            let sep: SpannedToken![::] = stream.parse()?;
            let field: SpannedToken![ident] = stream.parse()?;

            result = Self::FieldAccess {
                base: Box::new(Spanned::new(start, end, result)),
                sep,
                field,
            };
        }

        Ok(result)
    }
}

impl Peek for TypeExpr {
    fn peek(stream: &TokenStream) -> bool {
        // TypeExpr can start with an operator or a type reference
        TypeExprOp::peek(stream) || PathOrIdent::peek(stream)
    }
}

impl ToTokens for TypeExpr {
    fn write(
        &self,
        tt: &mut Printer,
    ) {
        match self {
            Self::TypeRef { reference } => reference.write(tt),
            Self::FieldAccess { base, sep, field } => {
                base.value.write(tt);
                tt.write(sep);
                tt.write(field);
            },
            Self::Op(op) => op.value.write(tt),
        }
    }
}

impl ToTokens for TypeExprOp {
    fn write(
        &self,
        tt: &mut Printer,
    ) {
        match self {
            Self::Pick { target, fields } => {
                tt.word("Pick[");
                target.write(tt);
                tt.word(", ");
                fields.write(tt);
                tt.word("]");
            },
            Self::Omit { target, fields } => {
                tt.word("Omit[");
                target.write(tt);
                tt.word(", ");
                fields.write(tt);
                tt.word("]");
            },
            Self::Partial { target, fields } => {
                tt.word("Partial[");
                target.write(tt);
                if let Some(f) = fields {
                    tt.word(", ");
                    f.write(tt);
                }
                tt.word("]");
            },
            Self::Required { target, fields } => {
                tt.word("Required[");
                target.write(tt);
                if let Some(f) = fields {
                    tt.word(", ");
                    f.write(tt);
                }
                tt.word("]");
            },
            Self::Exclude { target, variants } => {
                tt.word("Exclude[");
                target.write(tt);
                tt.word(", ");
                variants.write(tt);
                tt.word("]");
            },
            Self::Extract { target, variants } => {
                tt.word("Extract[");
                target.write(tt);
                tt.word(", ");
                variants.write(tt);
                tt.word("]");
            },
            Self::ArrayItem { target } => {
                tt.word("ArrayItem[");
                target.write(tt);
                tt.word("]");
            },
        }
    }
}

impl ToTokens for SelectorList {
    fn write(
        &self,
        tt: &mut Printer,
    ) {
        for (i, field) in self.fields.iter().enumerate() {
            if i > 0 {
                tt.word(" | ");
            }
            tt.write(&field.value);
        }
    }
}

impl ToTokens for VariantList {
    fn write(
        &self,
        tt: &mut Printer,
    ) {
        for (i, variant) in self.variants.iter().enumerate() {
            if i > 0 {
                tt.word(" | ");
            }
            tt.write(&variant.value);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tokens::tokenize;

    /// Parse a type expression from source code
    fn parse_type_expr(src: &str) -> AstResult<TypeExpr> {
        let mut tokens = tokenize(src).unwrap();
        TypeExpr::parse(&mut tokens)
    }

    mod type_ref {
        use super::*;

        #[test]
        fn simple_ident() {
            let expr = parse_type_expr("User").unwrap();
            assert!(expr.is_type_ref());
        }

        #[test]
        fn namespaced_path() {
            let expr = parse_type_expr("foo::bar::Baz").unwrap();
            assert!(expr.is_type_ref());
        }
    }

    mod field_access {
        use super::*;

        #[test]
        fn single_field() {
            // User::id is tokenized as a path - field access resolution happens in Phase 3.6
            let expr = parse_type_expr("User::id").unwrap();
            // This parses as a type reference with path - actual field access resolution is deferred
            assert!(expr.is_type_ref());
        }

        #[test]
        fn chained_fields() {
            // User::profile::avatar is tokenized as a path
            let expr = parse_type_expr("User::profile::avatar").unwrap();
            assert!(expr.is_type_ref());
        }
    }

    mod pick {
        use super::*;

        #[test]
        fn single_field() {
            let expr = parse_type_expr("Pick[User, id]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Pick { fields, .. } = &op.value {
                    assert_eq!(fields.fields.len(), 1);
                    assert_eq!(fields.fields[0].value.borrow_string(), "id");
                } else {
                    panic!("expected Pick operator");
                }
            }
        }

        #[test]
        fn multiple_fields() {
            let expr = parse_type_expr("Pick[User, id | name | email]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Pick { fields, .. } = &op.value {
                    assert_eq!(fields.fields.len(), 3);
                } else {
                    panic!("expected Pick operator");
                }
            }
        }
    }

    mod omit {
        use super::*;

        #[test]
        fn single_field() {
            let expr = parse_type_expr("Omit[User, password_hash]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Omit { fields, .. } = &op.value {
                    assert_eq!(fields.fields.len(), 1);
                } else {
                    panic!("expected Omit operator");
                }
            }
        }
    }

    mod partial {
        use super::*;

        #[test]
        fn all_fields() {
            let expr = parse_type_expr("Partial[User]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Partial { fields, .. } = &op.value {
                    assert!(fields.is_none());
                } else {
                    panic!("expected Partial operator");
                }
            }
        }

        #[test]
        fn specific_fields() {
            let expr = parse_type_expr("Partial[User, name | email]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Partial { fields, .. } = &op.value {
                    assert!(fields.is_some());
                    assert_eq!(fields.as_ref().unwrap().fields.len(), 2);
                } else {
                    panic!("expected Partial operator");
                }
            }
        }
    }

    mod required {
        use super::*;

        #[test]
        fn all_fields() {
            let expr = parse_type_expr("Required[UserInput]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Required { fields, .. } = &op.value {
                    assert!(fields.is_none());
                } else {
                    panic!("expected Required operator");
                }
            }
        }

        #[test]
        fn specific_fields() {
            let expr = parse_type_expr("Required[UserInput, id | name]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Required { fields, .. } = &op.value {
                    assert!(fields.is_some());
                    assert_eq!(fields.as_ref().unwrap().fields.len(), 2);
                } else {
                    panic!("expected Required operator");
                }
            }
        }
    }

    mod exclude {
        use super::*;

        #[test]
        fn single_variant() {
            let expr = parse_type_expr("Exclude[Response, Error]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Exclude { variants, .. } = &op.value {
                    assert_eq!(variants.variants.len(), 1);
                } else {
                    panic!("expected Exclude operator");
                }
            }
        }

        #[test]
        fn multiple_variants() {
            let expr = parse_type_expr("Exclude[Response, NotFound | Unauthorized | ServerError]")
                .unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Exclude { variants, .. } = &op.value {
                    assert_eq!(variants.variants.len(), 3);
                } else {
                    panic!("expected Exclude operator");
                }
            }
        }
    }

    mod extract {
        use super::*;

        #[test]
        fn single_variant() {
            let expr = parse_type_expr("Extract[Response, Success]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Extract { variants, .. } = &op.value {
                    assert_eq!(variants.variants.len(), 1);
                } else {
                    panic!("expected Extract operator");
                }
            }
        }
    }

    mod array_item {
        use super::*;

        #[test]
        fn simple() {
            let expr = parse_type_expr("ArrayItem[Users]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                assert!(matches!(op.value, TypeExprOp::ArrayItem { .. }));
            }
        }

        #[test]
        fn with_field_access() {
            // User::tags is tokenized as a path in the lexer
            let expr = parse_type_expr("ArrayItem[User::tags]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::ArrayItem { target } = &op.value {
                    // The target is a type ref with path - field access resolution is in Phase 3.6
                    assert!(target.is_type_ref());
                } else {
                    panic!("expected ArrayItem operator");
                }
            }
        }
    }

    mod nested_ops {
        use super::*;

        #[test]
        fn pick_then_partial() {
            let expr = parse_type_expr("Partial[Pick[User, name | email]]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Partial { target, .. } = &op.value {
                    assert!(target.is_op());
                } else {
                    panic!("expected Partial operator");
                }
            }
        }

        #[test]
        fn omit_then_required() {
            let expr = parse_type_expr("Required[Omit[User, password_hash]]").unwrap();
            assert!(expr.is_op());
            if let TypeExpr::Op(op) = &expr {
                if let TypeExprOp::Required { target, .. } = &op.value {
                    assert!(target.is_op());
                } else {
                    panic!("expected Required operator");
                }
            }
        }
    }
}
