mod debug;

use clap::Parser;
use rusty_funge::Funge;
use debug::FungeView;
use anyhow::Result;


#[derive(Parser)]
#[command(version)]
struct Args {
    #[arg(id = "funge code file")]
    input: String,
    #[arg(help = "debug, step on key press or steps / second",
          short, long, value_name = "interval", num_args = 0..=1)]
    debug: Option<Option<f64>>,
    #[arg(help = "number of bits in cell and funge values", short, long)]
    bits: Option<u8>,
    #[arg(help = "skip steps", short, long)]
    steps: Option<usize>,
    #[arg(id = "arguments to the funge (& or ~)")]
    arguments: Vec<String>,
}


macro_rules! run {
    ($a:expr, $i:ty) => {
        let mut funge = Funge::<$i>::from_file(&$a.input)?;
        match $a.debug {
            Some(interval) => {
                let mut funge = FungeView::new(funge, $a.arguments)?;
                if let Some(s) = $a.steps {
                    funge.step_n(s);
                }
                funge.debug(interval);
            }
            None => {
                funge = funge.with_arguments($a.arguments).run()?;
                std::process::exit(funge.return_code);
            }
        }
    }
}


fn main() -> Result<()> {
    let args = Args::parse();
    if let None = args.bits {
        run!(args, isize);
    } else if let Some(8) = args.bits {
        run!(args, i8);
    } else if let Some(16) = args.bits {
        run!(args, i16);
    } else if let Some(32) = args.bits {
        run!(args, i32);
    } else if let Some(64) = args.bits {
        run!(args, i64);
    } else if let Some(128) = args.bits {
        run!(args, i128);
    }
    Ok(())
}