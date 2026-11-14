use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group(0..10)]
    Animal,
    Dog = 20, // outside range
}

fn main() {}
