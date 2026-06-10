//! Helper (ignored by default): print initcode hex for the demo contracts so
//! they can be deployed manually with `cast`. Used to generate honest README
//! output; not part of the test suite.

mod common;

use common::*;

#[test]
#[ignore]
fn print_initcodes() {
    println!("TOKEN_INITCODE=0x{}", alloy_primitives::hex::encode(asm::initcode(&token_runtime())));
    println!(
        "REVERTER_INITCODE=0x{}",
        alloy_primitives::hex::encode(asm::initcode(&reverter_runtime()))
    );
    let var = |k: &str| -> Option<alloy_primitives::Address> {
        std::env::var(k).ok().and_then(|v| v.parse().ok())
    };
    if let (Some(eoa1), Some(token), Some(eoa2), Some(reverter)) =
        (var("EOA1"), var("TOKEN"), var("EOA2"), var("REVERTER"))
    {
        println!(
            "MAIN_INITCODE=0x{}",
            alloy_primitives::hex::encode(asm::initcode(&main_runtime(
                eoa1, token, eoa2, reverter
            )))
        );
    }
}
