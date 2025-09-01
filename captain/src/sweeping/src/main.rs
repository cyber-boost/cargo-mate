use anyhow::Result;
use clap::Parser;
mod away;
fn main() -> Result<()> {
    away::main()
}