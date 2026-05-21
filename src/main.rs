use std::fs::{File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn is_executable_block_start(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.contains("EXECUTABLE_BLOCK_START")
}

fn is_layer_change(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with(";LAYER_CHANGE") || trimmed.starts_with("; LAYER_CHANGE")
}

fn is_pause(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("PAUSE") || trimmed.starts_with("M25")
}

fn process_gcode_file<P: AsRef<Path>>(file_path: P) -> std::io::Result<()> {
    let file = File::open(&file_path)?;
    let reader = BufReader::new(file);

    let mut lines: Vec<String> = Vec::new();
    let mut first_layer_change: Option<usize> = None;
    let mut exec_block_start: Option<usize> = None;
    let mut pause_line: Option<usize> = None;
    let mut layer_change_before_pause: Option<usize> = None;

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        lines.push(line.clone());

        if is_executable_block_start(&line) && exec_block_start.is_none() {
            exec_block_start = Some(idx);
        }

        if is_layer_change(&line) {
            if first_layer_change.is_none() && exec_block_start.is_some() {
                first_layer_change = Some(idx);
            }
        }

        if is_pause(&line) {
            pause_line = Some(idx);
        }
    }

    if pause_line.is_none() {
        println!("Warning: PAUSE or M25 not found in {}", file_path.as_ref().display());
        return Ok(());
    }

    let pause_idx = pause_line.unwrap();
    let first_lc_idx = first_layer_change.unwrap_or(0);

    for i in (0..pause_idx).rev() {
        if is_layer_change(&lines[i]) {
            layer_change_before_pause = Some(i);
            break;
        }
    }

    if layer_change_before_pause.is_none() {
        println!("Warning: LAYER_CHANGE before PAUSE not found in {}", file_path.as_ref().display());
        return Ok(());
    }

    let last_lc_idx = layer_change_before_pause.unwrap();

    println!("Processed: {}", file_path.as_ref().display());
    println!("  - First LAYER_CHANGE at line {}", first_lc_idx + 1);
    println!("  - LAYER_CHANGE before PAUSE at line {}", last_lc_idx + 1);
    println!("  - PAUSE at line {}", pause_idx + 1);

    let mut output = Vec::new();

    for i in 0..first_lc_idx {
        let trimmed = lines[i].trim_start();
        if i > exec_block_start.unwrap_or(0) && (trimmed.starts_with("G28") || trimmed.starts_with("G1 ") || trimmed.starts_with("G1\t")) {
            continue;
        }
        output.push(lines[i].clone());
    }

    output.push(lines[first_lc_idx].clone());

    output.push(lines[last_lc_idx].clone());

    for i in (last_lc_idx + 1)..pause_idx {
        output.push(lines[i].clone());
    }

    for i in (pause_idx + 1)..lines.len() {
        output.push(lines[i].clone());
    }

    let mut file = File::create(&file_path)?;
    writeln!(file, "{}", output.join("\n"))?;

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <gcode_file_path>", args[0]);
        eprintln!("For Orca Slicer post-processing, set this in: Others > Post-processing Scripts");
        std::process::exit(1);
    }

    let file_path = &args[1];

    if let Err(e) = process_gcode_file(file_path) {
        eprintln!("Error processing {}: {}", file_path, e);
        std::process::exit(1);
    }

    println!("\nProcessing completed successfully.");
}
