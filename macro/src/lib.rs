//! `genum`: a proc-macro for declaring grouped, integer-repr enums with safe narrowing.
//!
//! Transforms a single "umbrella" enum into a set of tightly-typed sub-enums, each
//! representing one or more ranges of discriminants. Also generates `TryFrom`/`From`
//! conversions with a typed error enum.
//!
//! See [`genum`] for the full attribute documentation.

#![allow(clippy::doc_overindented_list_items)]

use proc_macro::TokenStream;
use quote::format_ident;
use syn::{Data, DeriveInput, parse_macro_input};

mod codegen;
mod literal;
mod model;
mod parse;
mod resolve;
mod validate;

use parse::{GenumArgs, collect_groups};

/// Transforms an umbrella enum into a set of grouped, `#[repr(T)]` enums with typed
/// narrowing conversions.
///
/// # Syntax
///
/// ```ignore
/// #[genum(repr_type)]
/// #[genum(repr_type, exhaustive)]
/// #[genum(repr_type, error = MyErrorName)]
/// pub enum Name {
///     #[group(start..end + start2..=end2 + single_value + ...)]
///     GroupName,
///
///     Variant1,
///     Variant2,
///     Variant3 = explicit_value,
///     Variant4,
///
///     #[group(...)]
///     OtherGroup,
///
///     Variant5,
/// }
/// ```
///
/// + `repr_type`: the integer representation (`u8`, `u16`, `u32`, `u64`,
///  `u128`, `usize`, `i8`, `i16`, `i32`, `i64`, `i128`, `isize`).
/// + `exhaustive`: disables auto-hole-filling; every group must be fully
///  populated.
/// + `error = Name`: overrides the default `{Umbrella}NarrowError` error enum
///  name.
/// + `#[group(...)]` on a variant declares a virtual group marker (the marker
///  itself is not emitted). Subsequent variants belong to that group.
/// + `Variant = value` forces a specific discriminant; the remaining auto
///  variants fill the free slots in ascending order.
///
/// The macro preserves doc-comments on the umbrella enum and on every regular
/// variant, in both the umbrella and group enums.
#[proc_macro_attribute]
pub fn genum(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as GenumArgs);
    let input = parse_macro_input!(item as DeriveInput);

    match genum_impl(args, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn genum_impl(
    GenumArgs {
        repr_type,
        exhaustive,
        error_name,
    }: GenumArgs,
    input: DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let enum_name = &input.ident;
    let vis = &input.vis;
    // preserve doc-comments and other attributes
    let input_attrs = input.attrs.clone();

    let error_name = error_name.unwrap_or_else(|| format_ident!("{}NarrowError", enum_name));

    let variants = match &input.data {
        Data::Enum(data) => &data.variants,
        _ => return Err(syn::Error::new_spanned(&input, "genum only works on enums")),
    };

    // Phase 1: parse.
    let mut groups = collect_groups(variants)?;

    // Phase 2: resolve forced/auto discriminants.
    for group in &mut groups {
        resolve::resolve_group_discriminants(group)?;
    }

    // Phase 3: validate overlaps & uniqueness.
    validate::validate_group_ranges(&groups)?;

    // Phase 3.5: validate that all range boundaries fit in the repr type.
    validate::validate_ranges_for_repr(&groups, &repr_type)?;

    // Phase 4: hole-filling or exhaustive check.
    if exhaustive {
        resolve::check_exhaustive(&groups)?;
    } else {
        let mut unknown_counter: u128 = 0;
        resolve::fill_holes(&mut groups, &mut unknown_counter);
    }

    // Phase 5: sort variants by discriminant for deterministic output.
    for group in &mut groups {
        group.variants.sort_by_key(|v| v.discriminant);
    }

    // Phase 6: codegen.
    let main_enum = codegen::gen_main_enum(enum_name, vis, &repr_type, &input_attrs, &groups)?;
    let group_enums = codegen::gen_group_enums(vis, &repr_type, &groups);
    let error_enum = codegen::gen_error_enum(vis, &error_name, enum_name, &groups);

    let is_known_impls = if exhaustive {
        quote::quote! {}
    } else {
        codegen::gen_is_known_impls(enum_name, vis, &repr_type, &groups)?
    };

    let try_from_impls = codegen::gen_try_from_impls(enum_name, &error_name, &repr_type, &groups)?;
    let from_impls = codegen::gen_from_impls(enum_name, &groups);

    Ok(quote::quote! {
        #main_enum
        #group_enums
        #is_known_impls
        #error_enum
        #try_from_impls
        #from_impls
    })
}
