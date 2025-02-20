use std::error::Error;

mod so1602a;
use so1602a::{SO1602A, SO1602A_ADDR};

fn main() -> Result<(), Box<dyn Error>> {
    let so1602a = SO1602A::new(SO1602A_ADDR)?;
    so1602a.setup()?;

    Ok(())
}
