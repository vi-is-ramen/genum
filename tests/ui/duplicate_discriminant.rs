use genum::genum;

#[genum(u32)]
pub enum Object {
    #[group(0..2 + 42)]
    Animal,
    Dog,
    Cat,
    Fox, // = 42

    #[group(42..44)] // 42 already taken
    Food,
    Egg,
}

fn main() {}
