use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group()]
    Animal,
}

fn main() {}
