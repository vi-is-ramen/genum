use genum::genum;

#[genum(f32)]
pub enum Object {
    #[group(0..10)]
    Animal,
    Dog,
}

fn main() {}
