use dobf::{DobfInstance, DobfConfig};
use std::error::Error;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,
    #[arg(short, long)]
    input: String,
    #[arg(short, long)]
    output: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Info).init()?;
    
    log::info!("version: v{}", option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"));

    let config = DobfConfig::new(&args.config)?;

    log::info!("Using config: {}", config.name);

    log::info!("Transforming file: {}", args.input);

    let instance = DobfInstance::new(&args.input)?;
    
    instance.load_config(config);

    // run all transforms
    instance.run()?;

    // save the patched file
    instance.save(args.output)?;

    Ok(())   
}
