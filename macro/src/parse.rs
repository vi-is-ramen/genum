use crate::model::{Group, GroupAttr, RangeElement, VariantInfo};
use syn::{
    parse::Parse, parse::ParseStream, punctuated::Punctuated, Attribute, Ident, Token, Type,
};

mod kw {
    syn::custom_keyword!(exhaustive);
    syn::custom_keyword!(error);
}

/// Arguments to the `#[genum(...)]` attribute.
///
/// Examples:
/// - `#[genum(u32)]`
/// - `#[genum(u32, exhaustive)]`
/// - `#[genum(u32, error = MyError)]`
/// - `#[genum(u32, exhaustive, error = MyError)]`
pub struct GenumArgs {
    pub repr_type: Type,
    pub exhaustive: bool,
    pub error_name: Option<Ident>,
}

impl Parse for GenumArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let repr_type: Type = input.parse()?;
        let mut exhaustive = false;
        let mut error_name: Option<Ident> = None;

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;

            let lookahead = input.lookahead1();
            if lookahead.peek(kw::exhaustive) {
                input.parse::<kw::exhaustive>()?;
                if exhaustive {
                    return Err(syn::Error::new(
                        input.span(),
                        "duplicate `exhaustive` argument",
                    ));
                }
                exhaustive = true;
            } else if lookahead.peek(kw::error) {
                input.parse::<kw::error>()?;
                input.parse::<Token![=]>()?;
                if error_name.is_some() {
                    return Err(syn::Error::new(input.span(), "duplicate `error` argument"));
                }
                error_name = Some(input.parse::<Ident>()?);
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(Self {
            repr_type,
            exhaustive,
            error_name,
        })
    }
}

impl Parse for GroupAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut elements = Vec::new();

        loop {
            let (first_val, _) = parse_int_literal(input)?;

            if input.peek(Token![..=]) {
                input.parse::<Token![..=]>()?;
                let (second_val, second_span) = parse_int_literal(input)?;
                if second_val < first_val {
                    return Err(syn::Error::new(
                        second_span,
                        "inclusive range end must be >= start",
                    ));
                }
                elements.push(RangeElement::Range {
                    start: first_val,
                    end: second_val + 1,
                });
            } else if input.peek(Token![..]) {
                input.parse::<Token![..]>()?;
                let (second_val, second_span) = parse_int_literal(input)?;
                if second_val <= first_val {
                    return Err(syn::Error::new(
                        second_span,
                        "range end must be strictly greater than start",
                    ));
                }
                elements.push(RangeElement::Range {
                    start: first_val,
                    end: second_val,
                });
            } else {
                elements.push(RangeElement::Single(first_val));
            }

            if input.peek(Token![+]) {
                input.parse::<Token![+]>()?;
            } else {
                break;
            }
        }

        if elements.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "group must contain at least one range or value",
            ));
        }

        Ok(Self { elements })
    }
}

/// Parse a possibly-negated integer literal from the input stream.
pub fn parse_int_literal(input: ParseStream) -> syn::Result<(i128, proc_macro2::Span)> {
    if input.peek(Token![-]) {
        let neg: Token![-] = input.parse()?;
        let lit: syn::LitInt = input.parse()?;
        let val: i128 = lit.base10_parse()?;
        Ok((-val, neg.span))
    } else {
        let lit: syn::LitInt = input.parse()?;
        let val: i128 = lit.base10_parse()?;
        Ok((val, lit.span()))
    }
}

/// Parse a possibly-negated integer literal from an `Expr`.
pub fn parse_int_expr(expr: &syn::Expr) -> syn::Result<i128> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(lit),
            ..
        }) => lit.base10_parse::<i128>(),
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr: inner,
            ..
        }) => {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(lit),
                ..
            }) = inner.as_ref()
            {
                let val: i128 = lit.base10_parse()?;
                Ok(-val)
            } else {
                Err(syn::Error::new_spanned(
                    inner,
                    "expected integer literal after unary minus",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            expr,
            "expected integer literal or negated integer literal",
        )),
    }
}

/// Walk the user-written variants and build the list of `Group`s.
///
/// A variant carrying `#[group(...)]` becomes a virtual group marker whose identifier
/// becomes the group name. All subsequent variants without `#[group]` belong to the
/// most recently opened group. Forced discriminants (`= value`) are captured but not
/// yet resolved — see `crate::resolve::resolve_group_discriminants`.
pub fn collect_groups(variants: &Punctuated<syn::Variant, Token![,]>) -> syn::Result<Vec<Group>> {
    let mut groups: Vec<Group> = Vec::new();
    let mut current_group: Option<usize> = None;

    for variant in variants {
        let mut group_attr: Option<GroupAttr> = None;
        let mut other_attrs: Vec<Attribute> = Vec::new();

        for attr in &variant.attrs {
            if attr.path().is_ident("group") {
                let parsed: GroupAttr = attr.parse_args()?;
                group_attr = Some(parsed);
            } else {
                other_attrs.push(attr.clone());
            }
        }

        let forced_discriminant: Option<i128> = match &variant.discriminant {
            Some((_, expr)) => Some(parse_int_expr(expr)?),
            None => None,
        };

        if let Some(ga) = group_attr {
            groups.push(Group {
                name: variant.ident.clone(),
                elements: ga.elements,
                variants: Vec::new(),
                marker_attrs: other_attrs, // preserve doc-comments from marker
            });
            current_group = Some(groups.len() - 1);
        } else {
            let Some(group_idx) = current_group else {
                return Err(syn::Error::new_spanned(
                    &variant.ident,
                    "Variant must belong to a group (rest variants are forbidden). \
                     Open a group first with a marker variant like `#[group(0..N)] GroupName,`.",
                ));
            };

            let group = &mut groups[group_idx];
            group.variants.push(VariantInfo {
                ident: variant.ident.clone(),
                discriminant: 0,
                attrs: other_attrs,
                is_unknown: false,
                forced_discriminant,
            });
        }
    }

    Ok(groups)
}
