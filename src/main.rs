use gerber2svg::Gerber2SVG;
use structopt::StructOpt;

// use crate::gerber_svg::GerberSVG;
// mod gerber_svg;

#[allow(dead_code)]
#[derive(Debug, StructOpt)]
#[structopt(name = "rusty-pcb", about = "Usage of rusty-pcb", version="0.1.0")]
struct Opt{
    /// The Gerber file
    #[structopt(short="-i", long="--input")]
    gerber_file: String,

    /// The SVG output file (otherwise SVG will be print on standard output)
    #[structopt(short="-o", long="--output")]
    svg_file: Option<String>,

    /// Crop the SVG to remove unnecessary space.
    #[structopt(short="-c", long="--crop")]
    crop: bool,

    /// Be more verbose and show gerber comments
    #[structopt(short="-v", long="--verbose")]
    verbose: bool,

    /// Be verbose and print debug info
    #[structopt(short="-d", long="--debug")]
    debug: bool,
}

pub fn main() -> Result<(), std::io::Error>{
    let opt = Opt::from_args();

    if opt.debug {
        simple_logger::init_with_level(log::Level::Debug).expect("The logger cannot be initialized.");
    }
    else if opt.verbose{
        simple_logger::init_with_level(log::Level::Info).expect("The logger cannot be initialized.");
    }
    else{
        simple_logger::init_with_level(log::Level::Warn).expect("The logger cannot be initialized.");
    }

    log::info!("Load gerber file...");
    let gerber = Gerber2SVG::from_file(opt.gerber_file.as_str())?;

    if opt.svg_file.is_some(){
        log::info!("Save SVG file...");
        gerber.save_svg(&opt.svg_file.unwrap().as_str(), opt.crop)?;
    }
    else {
        log::info!("Print SVG file...");
        println!("{}", gerber.to_string(opt.crop));
    }

    Ok(())
}
