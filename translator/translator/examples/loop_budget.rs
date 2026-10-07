use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(input) = args.next() else {
        eprintln!("usage: loop_budget <in.spv> [budget]");
        std::process::exit(2);
    };
    let budget = match args.next() {
        Some(text) => match text.parse::<u32>() {
            Ok(budget) => budget,
            Err(error) => {
                eprintln!("budget {text:?}: {error}");
                std::process::exit(2);
            }
        },
        None => metal2vulkan::DEFAULT_LOOP_BUDGET,
    };

    let spv = match std::fs::read(&input) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("read {input}: {error}");
            std::process::exit(1);
        }
    };

    let scratch = std::env::temp_dir();
    if let Err(error) = metal2vulkan::tools::spirv_val_bytes(&spv, Path::new(&scratch)) {
        eprintln!("input module does not validate before instrumentation: {error}");
        std::process::exit(1);
    }

    let (bounded, report) = match metal2vulkan::instrument_spirv_loop_budget(&spv, budget) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("instrument {input}: {error}");
            std::process::exit(1);
        }
    };

    println!(
        "{input}: {} loops bounded in place, {} via early return, {} non-iterating, {} -> {} bytes",
        report.loops_bounded_in_place,
        report.loops_bounded_via_early_return,
        report.loops_skipped,
        spv.len(),
        bounded.len()
    );

    match metal2vulkan::tools::spirv_val_bytes(&bounded, Path::new(&scratch)) {
        Ok(()) => println!("{input}: instrumented module PASSES spirv-val"),
        Err(error) => {
            eprintln!("{input}: instrumented module FAILS spirv-val: {error}");
            let dump = format!("{input}.bounded.spv");
            let _ = std::fs::write(&dump, &bounded);
            eprintln!("wrote {dump} for inspection");
            std::process::exit(1);
        }
    }

    if !report.had_loops() && bounded != spv {
        eprintln!("{input}: loop-free module was modified; the guard must be a no-op here");
        std::process::exit(1);
    }
}
