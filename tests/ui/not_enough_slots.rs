use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group(0..2)] // only 2 slots
    Animal,
    Dog,
    Cat,
    Bird, // not enough room
}

fn main() {}
