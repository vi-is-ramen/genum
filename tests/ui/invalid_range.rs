use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group(10..5)] // end < start
    Animal,
    Dog,
}

fn main() {}
