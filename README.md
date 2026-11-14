# genum

**Compile-time verified enum narrowing with zero-cost conversions and niche
optimizations.**

`genum` is a proc-macro that transforms a single "umbrella" enum into a family
of tightly-typed sub-enums: each owning a well-defined slice of the discriminant
space. You get safe narrowing, exhaustive conversions, and precise niche
optimization, all without a single runtime check where it doesn't belong.

```rust
# use genum::genum;
#[genum(u32)]
pub enum Token {
    #[group(0..16)]
    Operator,
    Plus, Minus, Star, Slash, Percent,

    #[group(32..64)]
    Literal,
    Integer, Float, String, Boolean,

    #[group(128..=129)]
    Control,
    Eof, Error,
}
```

This single declaration gives you:

+ `Token`: the umbrella, `u32`-backed;
+ `Operator`, `Literal`, `Control`: narrow sub-enums with their own type
  identity;
+ `TryFrom<Token> for Operator`: range-checked, returns a typed error on
  mismatch;
+ `From<Operator> for Token` — always safe, always free;
+ `Token::is_known()`: `true` for user-defined variants, `false` for reserved
  slots.

## Why `genum`

### Zero-cost narrowing

A `Token::Operator` is just a `Token` with a narrower type. Conversions are
single `transmute` calls: the optimizer sees right through them:

```rust,ignore
let tok: Token = /* ... */;

match tok.try_into() {
    Ok(op @ Operator::Plus) => /* handle plus */,
    Ok(op) => /* handle other operator */,
    Err(_) => /* not an operator */,
}
```

No heap, no boxing, no tag byte, no branch on the happy path.

### Niche optimization, pushed to the limit

Rust already optimizes `Option<NonZeroU32>` down to 4 bytes by stealing the `0`
niche. `genum` lets you do the same thing: but for **any** discriminant layout
you choose:

```rust
# use genum::genum;
#[genum(u8)]
pub enum Status {
    // Reserve the entire 0..=254 range for "valid" codes;
    // leave 255 as the "none" niche.
    #[group(0..=254)]
    Valid,
    Ok = 0,
    Created = 201,
    Accepted = 202,
    BadRequest = 400,
    NotFound = 404,
    Internal = 500,
    // ... 248 slots auto-filled with hidden UnknownN_ variants
}

// Option<Status> is exactly 1 byte. The compiler uses 255 as the None niche.
assert_eq!(std::mem::size_of::<Option<Status>>(), 1);
```

You decide which discriminants are "real" and which are reserved as invalid. The
macro fills the rest with `#[doc(hidden)]` variants that keep the layout stable
and make `transmute` sound: while leaving your chosen niche free for the
optimizer to exploit.

### Exhaustive or open: your call

For wire formats where every value means something:

```rust
# use genum::genum;
#[genum(u8, exhaustive)]
pub enum Opcode {
    #[group(0..4)]
    Arithmetic,
    Add, Sub, Mul, Div,

    #[group(4..8)]
    Logic,
    And, Or, Xor, Not,
}
// Compile error if any slot is left unfilled.
```

For forward-compatible protocols where unknown values must survive round-trips:

```rust
# use genum::genum;
#[genum(u8)]  // default: holes auto-filled
pub enum FrameType { /* ... */ }
// Unknown values become `UnknownN_` variants, not UB.
```

### Forced discriminants for ABI compatibility

When a protocol pins specific values:

```rust
# use genum::genum;
#[genum(u8)]
pub enum Sig {
    #[group(0..10)]
    Common,
    Term = 1,     // SIGTERM
    Int = 2,      // SIGINT
    Kill = 9,     // SIGKILL
    Usr1,         // auto: 0
    Usr2,         // auto: 3
    // remaining slots filled in order: 4, 5, 6, 7, 8
}
```

Forced values are reserved first; the rest fill ascending free slots in
declaration order. Out-of-range forced values, duplicates, and capacity
overflows are caught at compile time.

## Features at a glance

| Feature              | Syntax                                       |
|----------------------|----------------------------------------------|
| Half-open ranges     | `#[group(0..64)]`                            |
| Inclusive ranges     | `#[group(0..=63)]`                           |
| Single values        | `#[group(42)]`                               |
| Mixed elements       | `#[group(0..10 + 42 + 100..=105)]`           |
| Multiple groups      | just declare more `#[group]` markers         |
| Signed reprs         | `#[genum(i8)]` with `-10..5`                 |
| Forced discriminants | `Variant = 42`                               |
| Exhaustive mode      | `#[genum(u8, exhaustive)]`                   |
| Custom error name    | `#[genum(u32, error = BadToken)]`            |
| All integer reprs    | `u8`..`u128`, `i8`..`i128`, `usize`, `isize` |

## API

```rust,ignore
// Narrowing: Token -> Operator (fallible, range-checked)
let op: Operator = token.try_into()?;

// Widening: Operator -> Token (infallible)
let tok: Token = op.into();

// Introspection (non-exhaustive mode only)
if token.is_known() {
    // user-defined variant
} else {
    // UnknownN_: received a value we don't recognize, but can safely store
}
```

The error enum is generated with one `To{Group}` variant per group and
implements `Display` and `core::error::Error`:

```rust
pub enum TokenNarrowError {
    ToOperator,
    ToLiteral,
    ToControl,
}
```

## `no_std` support

`genum` itself is a pure proc-macro: it emits code that works in `#![no_std]`
out of the box. The generated enums only depend on `core`. The error enum
implements `core::error::Error`.

```rust
#![no_std]
# use genum::genum;
#[genum(u16)]
pub enum Irq { /* ... */ }
```

## MSRV

Rust **1.88** (edition **2024**).

## Safety

The macro uses `core::mem::transmute` internally, but every call site is sound
by construction:

+ `From<Group> for Umbrella`: every group discriminant is a member of the
  umbrella.
+ `TryFrom<Umbrella> for Group`: the range check runs **before** the transmute.
+ All enums share the same `#[repr(T)]` and derive `Clone + Copy`.
+ No `#[non_exhaustive]`: the variant set is the public API, which is exactly
  what enables niche optimizations.

You never write `unsafe` yourself.

## License

Licensed under either of

+ Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
+ MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for
inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual
licensed as above, without any additional terms or conditions.
