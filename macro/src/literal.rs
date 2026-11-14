use syn::Type;

/// Creates a literal suffixed for the requested `repr` type.
///
/// Returns a compile-time error if the value overflows the repr, or if the repr is
/// not a supported integer type. For `usize`/`isize`, values are conservatively
/// limited to the `u32`/`i32` range to remain portable across 32- and 64-bit targets.
pub fn make_literal(
    repr: &Type,
    value: i128,
    span: proc_macro2::Span,
) -> syn::Result<proc_macro2::Literal> {
    let repr_name = if let Type::Path(p) = repr {
        p.path.get_ident().map(|i| i.to_string())
    } else {
        None
    };

    let mut lit = match repr_name.as_deref() {
        Some("u8") => {
            check_unsigned(value, u8::MAX as i128, "u8", span)?;
            proc_macro2::Literal::u8_suffixed(value as u8)
        }
        Some("u16") => {
            check_unsigned(value, u16::MAX as i128, "u16", span)?;
            proc_macro2::Literal::u16_suffixed(value as u16)
        }
        Some("u32") => {
            check_unsigned(value, u32::MAX as i128, "u32", span)?;
            proc_macro2::Literal::u32_suffixed(value as u32)
        }
        Some("u64") => {
            check_unsigned(value, u64::MAX as i128, "u64", span)?;
            proc_macro2::Literal::u64_suffixed(value as u64)
        }
        Some("u128") => {
            if value < 0 {
                return Err(syn::Error::new(
                    span,
                    format!("value {} overflows u128", value),
                ));
            }
            proc_macro2::Literal::u128_suffixed(value as u128)
        }
        Some("usize") => {
            if value < 0 || value > u32::MAX as i128 {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "value {} overflows usize (limited to u32::MAX for portability)",
                        value
                    ),
                ));
            }
            proc_macro2::Literal::usize_suffixed(value as usize)
        }
        Some("i8") => {
            check_signed(value, i8::MIN as i128, i8::MAX as i128, "i8", span)?;
            proc_macro2::Literal::i8_suffixed(value as i8)
        }
        Some("i16") => {
            check_signed(value, i16::MIN as i128, i16::MAX as i128, "i16", span)?;
            proc_macro2::Literal::i16_suffixed(value as i16)
        }
        Some("i32") => {
            check_signed(value, i32::MIN as i128, i32::MAX as i128, "i32", span)?;
            proc_macro2::Literal::i32_suffixed(value as i32)
        }
        Some("i64") => {
            check_signed(value, i64::MIN as i128, i64::MAX as i128, "i64", span)?;
            proc_macro2::Literal::i64_suffixed(value as i64)
        }
        Some("i128") => proc_macro2::Literal::i128_suffixed(value),
        Some("isize") => {
            if value < i32::MIN as i128 || value > i32::MAX as i128 {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "value {} overflows isize (limited to i32 range for portability)",
                        value
                    ),
                ));
            }
            proc_macro2::Literal::isize_suffixed(value as isize)
        }
        Some(other) => {
            return Err(syn::Error::new_spanned(
                repr,
                format!(
                    "unsupported repr `{}`, expected one of: \
                     u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize",
                    other
                ),
            ));
        }
        None => {
            return Err(syn::Error::new_spanned(
                repr,
                "expected a primitive integer type as the first argument",
            ));
        }
    };

    lit.set_span(span);
    Ok(lit)
}

fn check_unsigned(value: i128, max: i128, name: &str, span: proc_macro2::Span) -> syn::Result<()> {
    if value < 0 || value > max {
        Err(syn::Error::new(
            span,
            format!("value {} overflows {}", value, name),
        ))
    } else {
        Ok(())
    }
}

fn check_signed(
    value: i128,
    min: i128,
    max: i128,
    name: &str,
    span: proc_macro2::Span,
) -> syn::Result<()> {
    if value < min || value > max {
        Err(syn::Error::new(
            span,
            format!("value {} overflows {}", value, name),
        ))
    } else {
        Ok(())
    }
}

/// Check that `value` fits in the given `repr` type.
///
/// Returns `Ok(())` if it fits, or a `syn::Error` with a descriptive message otherwise.
/// Used for early validation before codegen, so we can emit compile errors instead of panics.
pub fn check_value_fits_repr(repr: &Type, value: i128, span: proc_macro2::Span) -> syn::Result<()> {
    let repr_name = if let Type::Path(p) = repr {
        p.path.get_ident().map(|i| i.to_string())
    } else {
        None
    };

    match repr_name.as_deref() {
        Some("u8") => check_unsigned(value, u8::MAX as i128, "u8", span),
        Some("u16") => check_unsigned(value, u16::MAX as i128, "u16", span),
        Some("u32") => check_unsigned(value, u32::MAX as i128, "u32", span),
        Some("u64") => check_unsigned(value, u64::MAX as i128, "u64", span),
        Some("u128") => {
            if value < 0 {
                Err(syn::Error::new(
                    span,
                    format!("value {} overflows u128", value),
                ))
            } else {
                Ok(())
            }
        }
        Some("usize") => {
            if value < 0 || value > u32::MAX as i128 {
                Err(syn::Error::new(
                    span,
                    format!(
                        "value {} overflows usize (limited to u32::MAX for portability)",
                        value
                    ),
                ))
            } else {
                Ok(())
            }
        }
        Some("i8") => check_signed(value, i8::MIN as i128, i8::MAX as i128, "i8", span),
        Some("i16") => check_signed(value, i16::MIN as i128, i16::MAX as i128, "i16", span),
        Some("i32") => check_signed(value, i32::MIN as i128, i32::MAX as i128, "i32", span),
        Some("i64") => check_signed(value, i64::MIN as i128, i64::MAX as i128, "i64", span),
        Some("i128") => Ok(()),
        Some("isize") => {
            if value < i32::MIN as i128 || value > i32::MAX as i128 {
                Err(syn::Error::new(
                    span,
                    format!(
                        "value {} overflows isize (limited to i32 range for portability)",
                        value
                    ),
                ))
            } else {
                Ok(())
            }
        }
        Some(other) => Err(syn::Error::new_spanned(
            repr,
            format!(
                "unsupported repr `{}`, expected one of: \
                 u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize",
                other
            ),
        )),
        None => Err(syn::Error::new_spanned(
            repr,
            "expected a primitive integer type as the first argument",
        )),
    }
}
