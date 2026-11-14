use syn::{Attribute, Ident};

#[derive(Clone)]
pub enum RangeElement {
    Range { start: i128, end: i128 },
    Single(i128),
}

impl RangeElement {
    pub fn discriminants(&self) -> Vec<i128> {
        match self {
            RangeElement::Range { start, end } => (*start..*end).collect(),
            RangeElement::Single(d) => vec![*d],
        }
    }

    pub fn contains(&self, d: i128) -> bool {
        match self {
            RangeElement::Range { start, end } => d >= *start && d < *end,
            RangeElement::Single(s) => d == *s,
        }
    }

    pub fn to_inclusive(&self) -> DiscriminantRange {
        match self {
            RangeElement::Range { start, end } => DiscriminantRange {
                start: *start,
                end: end - 1,
            },
            RangeElement::Single(d) => DiscriminantRange { start: *d, end: *d },
        }
    }
}

pub struct GroupAttr {
    pub elements: Vec<RangeElement>,
}

#[derive(Clone)]
pub struct DiscriminantRange {
    pub start: i128,
    pub end: i128,
}

impl DiscriminantRange {
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

#[derive(Clone)]
pub struct VariantInfo {
    pub ident: Ident,
    pub discriminant: i128,
    pub attrs: Vec<Attribute>,
    pub is_unknown: bool,
    pub forced_discriminant: Option<i128>,
}

pub struct Group {
    pub name: Ident,
    pub elements: Vec<RangeElement>,
    pub variants: Vec<VariantInfo>,
    /// Attributes from the group marker variant (including doc-comments).
    /// These will be applied to the generated group enum.
    pub marker_attrs: Vec<Attribute>,
}

impl Group {
    pub fn all_discriminants(&self) -> Vec<i128> {
        self.elements
            .iter()
            .flat_map(RangeElement::discriminants)
            .collect()
    }

    pub fn contains(&self, d: i128) -> bool {
        self.elements.iter().any(|e| e.contains(d))
    }

    pub fn allowed_ranges(&self) -> Vec<DiscriminantRange> {
        self.elements
            .iter()
            .map(RangeElement::to_inclusive)
            .collect()
    }
}
