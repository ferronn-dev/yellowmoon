use clap::Parser;

#[derive(Parser)]
struct Cli {
  filename: String,
}

fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();
  println!("{:#?}", yellowmoon::undump::undump(&std::fs::read(cli.filename)?)?);
  Ok(())
}
