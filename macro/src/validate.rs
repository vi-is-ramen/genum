use crate::literal::check_value_fits_repr;
use crate::model::{DiscriminantRange, Group};
use syn::{Ident, Type};

/// Check that group ranges do not overlap and that no two variants (across any groups)
/// share a discriminant.
pub fn validate_group_ranges(groups: &[Group]) -> syn::Result<()> {
    // Group-vs-group overlap check.
    for (i, g1) in groups.iter().enumerate() {
        for g2 in &groups[i + 1..] {
            let ranges1 = g1.allowed_ranges();
            let ranges2 = g2.allowed_ranges();

            for r1 in &ranges1 {
                for r2 in &ranges2 {
                    if r1.overlaps(r2) {
                        return Err(syn::Error::new_spanned(
                            &g2.name,
                            format!(
                                "Group `{}` overlaps with group `{}`: range [{}..={}] intersects [{}..={}]",
                                g2.name, g1.name, r1.start, r1.end, r2.start, r2.end
                            ),
                        ));
                    }
                }
            }
        }
    }

    // Discriminant-level duplicate check.
    let mut all_discs: Vec<(i128, &Ident, &Ident)> = groups
        .iter()
        .flat_map(|g| {
            g.variants
                .iter()
                .map(move |v| (v.discriminant, &g.name, &v.ident))
        })
        .collect();

    all_discs.sort_by_key(|(d, _, _)| *d);

    for i in 1..all_discs.len() {
        if all_discs[i].0 == all_discs[i - 1].0 {
            let disc = all_discs[i].0;
            let (_, g1_name, v1_name) = &all_discs[i - 1];
            let (_, g2_name, v2_name) = &all_discs[i];
            return Err(syn::Error::new_spanned(
                v2_name,
                format!(
                    "Discriminant {disc} is claimed by both `{g1_name}::{v1_name}` and `{g2_name}::{v2_name}`"
                ),
            ));
        }
    }

    Ok(())
}

/// Merge overlapping/adjacent inclusive ranges into a minimal set.
///
/// The input is sorted by `start`. Adjacent ranges (where `cur.end + 1 == r.start`)
/// are merged into a single range.
pub fn merge_ranges(mut ranges: Vec<DiscriminantRange>) -> Vec<DiscriminantRange> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|r| r.start);

    let mut merged = Vec::with_capacity(ranges.len());
    let mut cur = ranges[0].clone();

    for r in ranges.into_iter().skip(1) {
        if r.start <= cur.end.saturating_add(1) {
            cur.end = cur.end.max(r.end);
        } else {
            merged.push(cur);
            cur = r;
        }
    }
    merged.push(cur);
    merged
}

/// Validate that every boundary value in every group's range elements fits in the
/// chosen `repr_type`. Runs before codegen so we emit compile errors rather than panics.
pub fn validate_ranges_for_repr(groups: &[Group], repr_type: &Type) -> syn::Result<()> {
    for group in groups {
        for element in &group.elements {
            // Check every boundary: for Range, check start and end-1 (last valid value);
            // for Single, check the single value.
            let values: Vec<i128> = match element {
                crate::model::RangeElement::Range { start, end } => {
                    vec![*start, end - 1]
                }
                crate::model::RangeElement::Single(d) => vec![*d],
            };

            for v in values {
                check_value_fits_repr(repr_type, v, group.name.span())?;
            }
        }
    }
    Ok(())
}
