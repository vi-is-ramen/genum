use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group(0..10)]
    Animal,
    Dog = 5,
    Cat = 5, // duplicate
}

fn main() {}
