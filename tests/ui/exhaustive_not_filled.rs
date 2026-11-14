use genum::genum;

#[genum(u32, exhaustive)]
pub enum Object {
    #[group(0..4)]
    Animal,
    Dog,
    Cat, // only 2 of 4 used
}

fn main() {}
