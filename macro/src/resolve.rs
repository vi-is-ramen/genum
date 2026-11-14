use crate::model::{Group, VariantInfo};
use quote::format_ident;
use std::collections::BTreeSet;
use syn::parse_quote;

/// Resolve discriminants for a group.
///
/// Forced discriminants (`= value`) are reserved first, validated against the
/// group's declared ranges, and checked for duplicates. The remaining (auto)
/// variants then fill the free slots in ascending order of discriminant value,
/// preserving their declaration order.
pub fn resolve_group_discriminants(group: &mut Group) -> syn::Result<()> {
    let all_slots: BTreeSet<i128> = group.all_discriminants().into_iter().collect();

    // Validate forced discriminants.
    let mut forced_seen: BTreeSet<i128> = BTreeSet::new();
    for v in &group.variants {
        if let Some(f) = v.forced_discriminant {
            if !group.contains(f) {
                return Err(syn::Error::new_spanned(
                    &v.ident,
                    format!(
                        "Forced discriminant {} is outside the ranges of group `{}`",
                        f, group.name
                    ),
                ));
            }
            if !forced_seen.insert(f) {
                return Err(syn::Error::new_spanned(
                    &v.ident,
                    format!(
                        "Duplicate forced discriminant {} in group `{}`",
                        f, group.name
                    ),
                ));
            }
        }
    }

    // Ensure there is enough room for auto variants.
    let auto_count = group
        .variants
        .iter()
        .filter(|v| v.forced_discriminant.is_none())
        .count();
    let free_count = all_slots.len() - forced_seen.len();
    if auto_count > free_count {
        return Err(syn::Error::new_spanned(
            &group.name,
            format!(
                "Not enough discriminant slots in group `{}`: {} auto variants but only {} free slots ({} forced, {} total)",
                group.name, auto_count, free_count, forced_seen.len(), all_slots.len()
            ),
        ));
    }

    // Distribute free slots to auto variants in ascending order.
    let free_slots: Vec<i128> = all_slots
        .iter()
        .copied()
        .filter(|s| !forced_seen.contains(s))
        .collect();

    let mut free_iter = free_slots.into_iter();

    for v in &mut group.variants {
        if let Some(f) = v.forced_discriminant {
            v.discriminant = f;
        } else {
            v.discriminant = free_iter.next().unwrap();
        }
    }

    Ok(())
}

/// Fill any remaining unoccupied slots in a group with hidden `UnknownN_` variants.
///
/// This is the default (non-exhaustive) behavior. The `counter` is shared across
/// groups so that `UnknownN_` names are globally unique within the umbrella enum.
pub fn fill_holes(groups: &mut [Group], counter: &mut u128) {
    for group in groups.iter_mut() {
        let used: BTreeSet<i128> = group.variants.iter().map(|v| v.discriminant).collect();
        let all_slots = group.all_discriminants();
        for disc in all_slots {
            if used.contains(&disc) {
                continue;
            }
            let ident = format_ident!("Unknown{}_", counter);
            *counter += 1;

            group.variants.push(VariantInfo {
                ident,
                discriminant: disc,
                attrs: vec![parse_quote!(#[doc(hidden)])],
                is_unknown: true,
                forced_discriminant: None,
            });
        }
    }
}

/// Check that every group is fully populated (used in exhaustive mode).
pub fn check_exhaustive(groups: &[Group]) -> syn::Result<()> {
    for group in groups {
        let used: BTreeSet<i128> = group.variants.iter().map(|v| v.discriminant).collect();
        let all_slots = group.all_discriminants();
        if used.len() != all_slots.len() {
            return Err(syn::Error::new_spanned(
                &group.name,
                format!(
                    "Group `{}` is not fully filled: used {} of {} discriminants (exhaustive mode)",
                    group.name,
                    used.len(),
                    all_slots.len()
                ),
            ));
        }
    }
    Ok(())
}
