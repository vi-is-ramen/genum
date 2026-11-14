use genum::genum;

#[genum(u32)]
pub enum Object {
    Dog, // error: variant outside group
}

fn main() {}
