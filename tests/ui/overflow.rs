use genum::genum;

#[genum(u8)]
pub enum Object {
    #[group(0..300)] // overflow u8
    Animal,
    Dog,
}

fn main() {}
