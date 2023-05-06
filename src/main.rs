#[macro_use]
extern crate num_derive;

mod constants;
mod metrics;
mod renderer;
mod util;

use crate::renderer::section::{sum_section_heights, Section};
use crate::renderer::TerminalRenderer;
use gumdrop::Options;


use crossterm::{
    cursor, execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use futures::executor::block_on;
// use metrics::histogram::load_zenith_store;
use std::error::Error;
// use std::fs;
use std::io::stdout;
// use std::panic;
// use std::panic::PanicInfo;
// use std::path::Path;
// use std::time::Duration;
// use std::time::SystemTime;
use std::process::{Command,exit};
//use std::path::PathBuf;
use dirs;
use execute::Execute;


// fn panic_hook(info: &PanicInfo<'_>) {
//     let location = info.location().unwrap(); // The current implementation always returns Some
//     let msg = match info.payload().downcast_ref::<&'static str>() {
//         Some(s) => *s,
//         None => match info.payload().downcast_ref::<String>() {
//             Some(s) => &s[..],
//             None => "Box<Any>",
//         },
//     };
//     error!("thread '<unnamed>' panicked at '{}', {}\r", msg, location);
//     restore_terminal();
//     println!("thread '<unnamed>' panicked at '{}', {}\r", msg, location);
// }

fn init_terminal() {
    let mut sout = stdout();
    execute!(sout, EnterAlternateScreen).expect("Unable to enter alternate screen");
    execute!(sout, cursor::Hide).expect("Unable to hide cursor");
    execute!(sout, Clear(ClearType::All)).expect("Unable to clear screen.");
    enable_raw_mode().expect("Unable to enter raw mode.");
}

fn restore_terminal() {
    let mut sout = stdout();
    // Restore cursor position and clear screen for TTYs
    execute!(sout, cursor::MoveTo(0, 0)).expect("Attempt to write to alternate screen failed.");
    execute!(sout, Clear(ClearType::All)).expect("Unable to clear screen.");
    execute!(sout, LeaveAlternateScreen).expect("Unable to leave alternate screen.");
    execute!(sout, cursor::Show).expect("Unable to restore cursor.");
    disable_raw_mode().expect("Unable to disable raw mode");
}

// fn use_db_history(db_path: &Path, tick_rate: u64) -> Option<bool> {
//     let db_path = db_path.join("store");
//     if db_path.exists() {
//         let now = SystemTime::now();
//         let map = match load_zenith_store(&db_path, &now) {
//             Some(map) => map,
//             None => return Some(true),
//         };
//         let tick = Duration::from_millis(tick_rate);
//         if map.tick != tick {
//             println!(
//                 "Using Database:{:?}\nDatabase tick: {} does not match supplied tick: {}.",
//                 db_path,
//                 map.tick.as_millis(),
//                 tick.as_millis()
//             );
//             println!("Proceed with (D)atabase tick: {}, (S)upplied tick with history disabled: {}, (E)xit?:", map.tick.as_millis(), tick.as_millis());
//             let mut line = String::new();
//             let _num_characters = std::io::stdin().read_line(&mut line).unwrap_or_default();
//             match line.as_bytes()[0].to_ascii_lowercase() {
//                 b'd' => Some(true),
//                 b's' => Some(false),
//                 b'e' => None,
//                 _ => None,
//             }
//         } else {
//             Some(true)
//         }
//     } else {
//         Some(true)
//     }
// }

macro_rules! push_geometry {
    ($geom:expr, $section:expr, $height:expr) => {
        if $height > 0 {
            $geom.push(($section, $height as f64));
        }
    };
}

macro_rules! exit_with_message {
    ($msg:expr, $code:expr) => {
        restore_terminal();
        println!("{}", $msg);
        exit($code);
    };
}

fn create_geometry(
    system_info_height: u16,
    process_height: u16,
    // net_height: u16,
    // disk_height: u16,
    // sensor_height: u16,
    // graphics_height: u16,
) -> Vec<(Section, f64)> {
    let mut geometry: Vec<(Section, f64)> = Vec::new();
    push_geometry!(geometry, Section::SystemInfo, system_info_height);
    push_geometry!(geometry, Section::Process, process_height);
    // push_geometry!(geometry, Section::Cpu, cpu_height);
    // push_geometry!(geometry, Section::Network, net_height);
    // push_geometry!(geometry, Section::Disk, disk_height);
    // push_geometry!(geometry, Section::Graphics, graphics_height);
    // assert_eq!(sensor_height, 0); // not implemented

    if geometry.is_empty() {
        exit_with_message!("All sections have size specified as zero!", 1);
    }
    // sum of minimum percentages should not exceed 100%
    let sum_heights = sum_section_heights(&geometry);
    // 100.1 to account for possible float precision error
    if sum_heights > 100.1 {
        let msg = format!(
            "Sum of minimum percent heights cannot exceed 100 but was {:}.",
            sum_heights
        );
        exit_with_message!(msg, 1);
    }
    // distribute the remaining percentage proportionately among the non-zero ones
    let factor = 100.0 / sum_heights;
    if factor > 1.0 {
        geometry.iter_mut().for_each(|s| s.1 *= factor);
    }
    // after redistribution, the new sum should be 100% with some tolerance for precision error
    let new_sum_heights = sum_section_heights(&geometry);
    assert!((99.9..=100.1).contains(&new_sum_heights));

    geometry
}

fn start_elmodeer(
    rate: u64,
    system_info_height: u16,
    process_height: u16,
    // net_height: u16,
    // disk_height: u16,
    // sensor_height: u16,
    // graphics_height: u16,
    // disable_history: bool,
    // db_path: &str,
) -> Result<(), Box<dyn Error>> {
    // debug!("Starting with Arguments: rate: {}, cpu: {}, net: {}, disk: {}, process: {}, graphics: {}, disable_history: {}, db_path: {}",
    //       rate,
    //       cpu_height,
    //       net_height,
    //       disk_height,
    //       process_height,
    //       graphics_height,
    //       disable_history,
    //       db_path,
    // );

    // let db_path = Path::new(db_path);

    // let use_history = false;
    // if disable_history {
    //     false
    // } else {
    //     match use_db_history(db_path, rate) {
    //         Some(r) => r,
    //         None => {
    //             exit(0);
    //         }
    //     }
    // };

    init_terminal();

    // setup a panic hook so we can see our panic messages.
    // panic::set_hook(Box::new(|info| {
    //     panic_hook(info);
    // }));

    // get pid before runtime start, so we always get the main pid and not the tid of a thread
    // let main_pid = std::process::id();

    let run = || async {
        //check lock
        // let (db, lock) = (None, None);

        // if use_history {
        //     let db_path = Path::new(db_path);

        //     if !db_path.exists() {
        //         debug!("Creating DB dir.");
        //         fs::create_dir_all(db_path).expect("Couldn't Create DB dir.");
        //     } else {
        //         if !db_path.is_dir() {
        //             exit_with_message!(
        //                 format!(
        //                     "Expected the path to be a directory not a file: {}",
        //                     db_path.to_string_lossy()
        //                 ),
        //                 1
        //             );
        //         }
        //     }
        //     debug!("Creating Lock");

        //     let lock_path = db_path.join(".zenith.lock");
        //     match util::Lockfile::new(main_pid, &lock_path).await {
        //         Some(f) => (Some(db_path.to_owned()), Some(f)), // keeps the lock handle alive
        //         None => {
        //             warn!(
        //                 "{:} exists and history recording is on. Is another copy of zenith \
        //                     open? If not remove the path and open zenith again.",
        //                 lock_path.display()
        //             );
        //             (None, None)
        //         }
        //     }
        // } else {
        //     (None, None)
        // };

        // debug!("Create Renderer");

        let geometry: Vec<(Section, f64)> = create_geometry(
            system_info_height,
            process_height,
            // net_height,
            // disk_height,
            // sensor_height,
            // graphics_height,
        );
        let mut r = TerminalRenderer::new(rate, &geometry);

        r.start().await;

        // only drop lock at the end
        // drop(lock);
    };

    block_on(run());

    // debug!("Shutting Down.");

    restore_terminal();

    Ok(())
}

fn validate_refresh_rate(arg: &str) -> Result<u64, String> {
    let val = arg.parse::<u64>().map_err(|e| e.to_string())?;
    if val >= 1000 {
        Ok(val)
    } else {
        Err(format!(
            "{} Enter a refresh rate that is at least 1000 ms",
            arg
        ))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
   
    let args = std::env::args().collect::<Vec<_>>();
    let opts =
        MOptions::parse_args_default(&args[1..]).map_err(|e| format!("{}: {}", args[0], e))?;
    
    if opts.tree == true {
        let mut home_dir = dirs::home_dir().expect("Failed to get home directory");
        home_dir.push("el-modeer/modeer");
        let home_dir_str = home_dir.to_string_lossy().to_string();
        // println!("Home directory with Modeer: {}", home_dir_str);
        let mut tree_command = Command::new(&home_dir_str);

        if tree_command.execute_check_exit_status_code(0).is_err() {
            eprintln!("The path `{}` is not a correct executable binary file.", home_dir_str);
        }    
        exit(0);
    }

    

    // TODO: Add help description.
    if opts.help_requested() {
        println!(
            "El-Modeer {}
Abdallah Abdelaziz <abdallah_taha@aucegypt>, 
Amer Elsheikh <amer.elsheikh@aucegypt>, 
Gehad Fekry <gehadsalemfekry@aucegypt>.
El-Modeer, sort of like top but in rust.
Up/down arrow keys move around the process table. Return (enter) will focus on a process.

Usage: {} [OPTIONS]

{}
",
            env!("CARGO_PKG_VERSION"),
            args[0],
            MOptions::usage()
        );
        return Ok(());
    } else if opts.version {
        println!("el-modeer {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    start_elmodeer(
        opts.refresh_rate,
        opts.system_info_height, 
        opts.process_height,
        // 0,
        // 0,
        // 0,
        // 0,
        // false,
        // &"",
    )
}

#[derive(Options)]
struct MOptions {
    /// Disables history when flag is present
    // #[options(no_short, long = "disable-history", default = "false")]
    // disable_history: bool,

    #[options()]
    help: bool,

    #[options(short = "V")]
    version: bool,

    /// Min Percent Height of CPU/Memory visualization.
    // #[options(short = "c", long = "cpu-height", default = "17", meta = "INT")]
    // cpu_height: u16,


    /// Database to use, if any.
    // #[options(no_short, default_expr = "default_db_path()", meta = "STRING")]
    // db: String,
    
    /// Min Percent Height of Disk visualization.
    // #[options(short = "d", long = "disk-height", default = "17", meta = "INT")]
    // disk_height: u16,
    
    /// Min Percent Height of Network visualization.
    // #[options(short = "n", long = "net-height", default = "17", meta = "INT")]
    // net_height: u16,
    
    /// Min Percent Height of CPU/Memory visualization.
    #[options(short = "s", long = "system-info-height", default = "7", meta = "INT")]
    system_info_height: u16,

    /// Min Percent Height of Process Table.
    #[options(short = "p", long = "process-height", default = "32", meta = "INT")]
    process_height: u16,

    /// Refresh rate in milliseconds.
    #[options(
        short = "r",
        long = "refresh-rate",
        default = "2000",
        parse(try_from_str = "validate_refresh_rate"),
        meta = "INT"
    )]
    refresh_rate: u64,

    /// Start GUI tree
    #[options(
        short = "t",
        long = "tree"
    )]
    tree: bool,

    // Min Percent Height of Graphics Card visualization.
    // #[cfg(feature = "nvidia")]
    // #[options(short = "g", long = "graphics-height", default = "17", meta = "INT")]
    // graphics_height: u16,
}
