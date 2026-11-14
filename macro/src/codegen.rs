use crate::literal::make_literal;
use crate::model::{DiscriminantRange, Group, VariantInfo};
use crate::validate::merge_ranges;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Type, Visibility};

/// Generate the umbrella enum, preserving doc-comments and other attributes that
/// were applied to the user-written `enum` declaration.
pub fn gen_main_enum(
    enum_name: &Ident,
    vis: &Visibility,
    repr_type: &Type,
    input_attrs: &[syn::Attribute],
    groups: &[Group],
) -> syn::Result<TokenStream> {
    let main_variants: Vec<_> = groups
        .iter()
        .flat_map(|group| {
            group.variants.iter().map(|v| {
                let ident = &v.ident;
                let attrs = &v.attrs;
                let disc_lit = make_literal(repr_type, v.discriminant, ident.span())
                    .expect("discriminant out of range was already checked");
                quote! {
                    #(#attrs)*
                    #ident = #disc_lit
                }
            })
        })
        .collect();

    Ok(quote! {
        #(#input_attrs)*
        #[repr(#repr_type)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #vis enum #enum_name {
            #(#main_variants),*
        }
    })
}

/// Generate one `#[repr(T)]` enum per group.
pub fn gen_group_enums(vis: &Visibility, repr_type: &Type, groups: &[Group]) -> TokenStream {
    let group_enums = groups.iter().map(|group| {
        let group_name = &group.name;
        let marker_attrs = &group.marker_attrs;
        let variants = group.variants.iter().map(|v| {
            let ident = &v.ident;
            let attrs = &v.attrs;
            let disc_lit = make_literal(repr_type, v.discriminant, ident.span())
                .expect("discriminant out of range was already checked");
            quote! {
                #(#attrs)*
                #ident = #disc_lit
            }
        });

        quote! {
            #(#marker_attrs)*
            #[repr(#repr_type)]
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            #vis enum #group_name {
                #(#variants),*
            }
        }
    });

    quote! { #(#group_enums)* }
}

/// Generate the error enum, its `Display` impl, and a blanket `core::error::Error` impl.
pub fn gen_error_enum(
    vis: &Visibility,
    error_name: &Ident,
    enum_name: &Ident,
    groups: &[Group],
) -> TokenStream {
    let mut error_variants = Vec::new();
    let mut error_display_arms = Vec::new();

    for group in groups {
        let group_name = &group.name;
        let variant_name = format_ident!("To{group_name}");
        let obj_str = enum_name.to_string();
        let group_str = group_name.to_string();
        let msg = format!("{obj_str} -> {group_str}");

        error_variants.push(quote! { #variant_name });
        error_display_arms.push(quote! {
            Self::#variant_name => f.write_str(#msg)
        });
    }

    quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #vis enum #error_name {
            #(#error_variants),*
        }

        impl core::fmt::Display for #error_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                match self {
                    #(#error_display_arms),*
                }
            }
        }

        impl core::error::Error for #error_name {}
    }
}

/// Generate `impl #enum_name { fn is_known(&self) -> bool }` and one per group.
///
/// The body is an optimized range-check over every user-defined variant's discriminant,
/// merged into the minimal covering set of inclusive ranges. Only emitted in
/// non-exhaustive mode.
pub fn gen_is_known_impls(
    enum_name: &Ident,
    vis: &Visibility,
    repr_type: &Type,
    groups: &[Group],
) -> syn::Result<TokenStream> {
    let all_user_variants: Vec<VariantInfo> = groups
        .iter()
        .flat_map(|g| g.variants.iter().cloned())
        .collect();

    let umbrella_impl = gen_one_is_known_impl(enum_name, vis, &all_user_variants, repr_type)?;

    let group_impls = groups
        .iter()
        .map(|group| gen_one_is_known_impl(&group.name, vis, &group.variants, repr_type))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #umbrella_impl
        #(#group_impls)*
    })
}

fn gen_one_is_known_impl(
    enum_name: &Ident,
    vis: &Visibility,
    variants: &[VariantInfo],
    repr_type: &Type,
) -> syn::Result<TokenStream> {
    let ranges: Vec<DiscriminantRange> = variants
        .iter()
        .filter(|v| !v.is_unknown)
        .map(|v| DiscriminantRange {
            start: v.discriminant,
            end: v.discriminant,
        })
        .collect();

    let merged = merge_ranges(ranges);

    let check_expr = gen_range_check(
        quote! { *self as #repr_type },
        &merged,
        repr_type,
        enum_name.span(),
    )?;

    Ok(quote! {
        impl #enum_name {
            /// Returns `true` if this value is a user-defined variant, `false` if it's
            /// an auto-generated `UnknownN_` variant used for hole-filling.
            ///
            /// This method is only available in non-exhaustive mode and is implemented
            /// as an optimized range check rather than a match statement.
            #[inline]
            #vis fn is_known(&self) -> bool {
                #check_expr
            }
        }
    })
}

/// Generate `TryFrom<Umbrella> for Group` impls.
pub fn gen_try_from_impls(
    enum_name: &Ident,
    error_name: &Ident,
    repr_type: &Type,
    groups: &[Group],
) -> syn::Result<TokenStream> {
    let impls = groups
        .iter()
        .map(|group| {
            let group_name = &group.name;
            let error_variant = format_ident!("To{}", group_name);
            let allowed = group.allowed_ranges();

            let check_expr =
                gen_range_check(quote! { val }, &allowed, repr_type, group_name.span())?;

            Ok(quote! {
                impl TryFrom<#enum_name> for #group_name {
                    type Error = #error_name;

                    #[allow(unused_comparisons)]
                    fn try_from(value: #enum_name) -> Result<Self, Self::Error> {
                        let val = value as #repr_type;
                        if #check_expr {
                            unsafe { Ok(core::mem::transmute(value)) }
                        } else {
                            Err(#error_name::#error_variant)
                        }
                    }
                }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! { #(#impls)* })
}

/// Generate `From<Group> for Umbrella` impls (always safe).
pub fn gen_from_impls(enum_name: &Ident, groups: &[Group]) -> TokenStream {
    let impls = groups.iter().map(|group| {
        let group_name = &group.name;
        quote! {
            impl From<#group_name> for #enum_name {
                fn from(value: #group_name) -> Self {
                    unsafe { core::mem::transmute(value) }
                }
            }
        }
    });

    quote! { #(#impls)* }
}

/// Build a boolean expression that is true iff `val_expr` falls inside any of the
/// given ranges. Produces `(val >= a && val <= b) || (val == c) || ...`.
fn gen_range_check(
    val_expr: TokenStream,
    ranges: &[DiscriminantRange],
    repr_type: &Type,
    span: proc_macro2::Span,
) -> syn::Result<TokenStream> {
    if ranges.is_empty() {
        return Ok(quote! { false });
    }

    let conditions: Vec<_> = ranges
        .iter()
        .map(|range| {
            let start_lit = make_literal(repr_type, range.start, span)?;
            let end_lit = make_literal(repr_type, range.end, span)?;

            Ok(if range.start == range.end {
                quote! { (#val_expr == #start_lit) }
            } else {
                quote! { (#val_expr >= #start_lit && #val_expr <= #end_lit) }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! { #(#conditions)||* })
}
