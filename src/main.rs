mod debug;

use std::error::Error;
use clap::Parser;
use befunge::Funge;
use debug::FungeView;


#[derive(Parser)]
#[command(version)]
struct Args {
    #[arg(id = "funge code file")]
    input: String,
    #[arg(help = "debug, step on key press or steps / second",
          short, long, value_name = "interval", num_args = 0..=1)]
    debug: Option<Option<f64>>,
    #[arg(id = "arguments to the funge (& or ~)")]
    arguments: Vec<String>,
}


fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let mut funge = Funge::from_file(&args.input)?;
    if args.arguments.len() > 0 {
        funge = funge.with_inputs(args.arguments)?;
    }

    match args.debug {
        Some(interval) => FungeView::new(funge)?.debug(interval)?,
        None => { funge.run()?; }
    }
    Ok(())
}