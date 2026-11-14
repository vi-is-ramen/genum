use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group(0..10)]
    Animal,
    Dog,

    #[group(5..15)] // overlaps with Animal
    Food,
    Egg,
}

fn main() {}
