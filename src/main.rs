use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockType {
    Support,
    OuterWall,
    InnerWall,
    InternalSolidInfill,
    SparseInfill,
}

impl BlockType {
    fn from_arg(arg: &str) -> Option<Self> {
        match arg {
            "--支撑" | "--Support" => Some(BlockType::Support),
            "--外墙" | "--OuterWall" => Some(BlockType::OuterWall),
            "--内墙" | "--InnerWall" => Some(BlockType::InnerWall),
            "--实心填充" | "--SolidInfill" => Some(BlockType::InternalSolidInfill),
            "--稀疏填充" | "--SparseInfill" => Some(BlockType::SparseInfill),
            _ => None,
        }
    }

    fn type_name(&self) -> &str {
        match self {
            BlockType::Support => "Support",
            BlockType::OuterWall => "Outer wall",
            BlockType::InnerWall => "Inner wall",
            BlockType::InternalSolidInfill => "Internal solid infill",
            BlockType::SparseInfill => "Sparse infill",
        }
    }
}

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
    trimmed.starts_with("PAUSE")
        || trimmed.starts_with("M601")
        || trimmed.starts_with("CONTINUE")
        || trimmed.starts_with("continue")
        || trimmed.starts_with("接续")
        || trimmed.starts_with("继续")
}

fn is_type_line(line: &str, type_name: &str) -> bool {
    let trimmed = line.trim_start();
    let prefix = format!(";TYPE:{}", type_name);
    trimmed.starts_with(&prefix)
}

fn is_any_type(line: &str) -> bool {
    line.trim_start().starts_with(";TYPE:")
}

fn process_gcode_file<P>(file_path: P, block_type: Option<BlockType>) -> std::io::Result<()>
where
    P: AsRef<Path>,
{
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
        println!(
            "抱歉未找到暂停指令(PAUSE/M601/CONTINUE/continue/接续/继续)在 {} 中",
            file_path.as_ref().display()
        );
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
        println!(
            "抱歉 PAUSE 之前的 LAYER_CHANGE 没有在 {} 中找到",
            file_path.as_ref().display()
        );
        return Ok(());
    }

    let last_lc_idx = layer_change_before_pause.unwrap();

    let next_layer_change_after_pause: Option<usize> = {
        let mut result = None;
        for i in (pause_idx + 1)..lines.len() {
            if is_layer_change(&lines[i]) {
                result = Some(i);
                break;
            }
        }
        result
    };

    let layer_end_idx = next_layer_change_after_pause.unwrap_or(lines.len());

    println!("处理文件: {}", file_path.as_ref().display());
    println!("  - 第一个 LAYER_CHANGE 行在第 {} 行", first_lc_idx + 1);
    println!("  - PAUSE 之前的 LAYER_CHANGE 行在第 {} 行", last_lc_idx + 1);
    println!("  - PAUSE 行在第 {} 行", pause_idx + 1);

    let mut output = vec![];

    for i in 0..first_lc_idx {
        let trimmed = lines[i].trim_start();
        if i > exec_block_start.unwrap_or(0)
            && (trimmed.starts_with("G28") || trimmed.starts_with("G1 "))
        {
            continue;
        }
        output.push(lines[i].clone());
    }

    output.push(lines[first_lc_idx].clone());
    output.push(lines[last_lc_idx].clone());

    if let Some(bt) = block_type {
        let target_type = bt.type_name();
        let mut type_indices: Vec<usize> = Vec::new();

        for i in (last_lc_idx + 1)..layer_end_idx {
            if is_any_type(&lines[i]) {
                type_indices.push(i);
            }
        }

        if type_indices.is_empty() {
            println!("  - 警告: 在 PAUSE 所在层未找到任何 TYPE 块，仅移除 PAUSE");
            for i in (last_lc_idx + 1)..layer_end_idx {
                if i == pause_idx {
                    continue;
                }
                output.push(lines[i].clone());
            }
        } else {
            let matching_pos = type_indices
                .iter()
                .position(|&i| is_type_line(&lines[i], target_type));

            if let Some(match_pos) = matching_pos {
            let remove_start = type_indices[0];
            let remove_end = if match_pos + 1 < type_indices.len() {
                type_indices[match_pos + 1]
            } else {
                layer_end_idx
            };

            let start_type_name = lines[type_indices[0]]
                .trim_start()
                .strip_prefix(";TYPE:")
                .unwrap_or("unknown");
            let end_type_name = lines[type_indices[match_pos]]
                .trim_start()
                .strip_prefix(";TYPE:")
                .unwrap_or("unknown");
            println!(
                "  - 删除 TYPE 块: 从第 {} 个 TYPE ({}) 到第 {} 个 TYPE ({})",
                type_indices[0] + 1,
                start_type_name,
                type_indices[match_pos] + 1,
                end_type_name
            );

            for i in (last_lc_idx + 1)..layer_end_idx {
                if i == pause_idx {
                    continue;
                }
                if i >= remove_start && i < remove_end {
                    continue;
                }
                output.push(lines[i].clone());
            }
        } else {
            println!(
                "  - 警告: 在 PAUSE 所在层未找到 TYPE:{} 块，仅移除 PAUSE",
                target_type
            );
            for i in (last_lc_idx + 1)..layer_end_idx {
                if i == pause_idx {
                    continue;
                }
                output.push(lines[i].clone());
            }
        }
        }
    } else {
        for i in (last_lc_idx + 1)..pause_idx {
            output.push(lines[i].clone());
        }
    }

    if block_type.is_some() {
        for i in layer_end_idx..lines.len() {
            output.push(lines[i].clone());
        }
    } else {
        for i in (pause_idx + 1)..lines.len() {
            output.push(lines[i].clone());
        }
    }

    let mut file = File::create(&file_path)?;
    writeln!(file, "{}", output.join("\n"))?;

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "使用方法: {} <G代码文件路径> [--支撑|--外墙|--内墙|--实心填充|--稀疏填充]",
            args[0]
        );
        eprintln!("英文别名: [--Support|--OuterWall|--InnerWall|--SolidInfill|--SparseInfill]");
        eprintln!("在 Orca Slicer 中设置后处理脚本: 其他选项 > 后处理脚本 中添加此脚本");
        std::process::exit(1);
    }

    let mut file_path: Option<String> = None;
    let mut block_type: Option<BlockType> = None;

    for arg in &args[1..] {
        if let Some(bt) = BlockType::from_arg(arg) {
            if block_type.is_some() {
                eprintln!("错误: 类型参数两两互斥，只能指定其中一个");
                std::process::exit(1);
            }
            block_type = Some(bt);
        } else {
            file_path = Some(arg.clone());
        }
    }

    let file_path = file_path.unwrap_or_else(|| {
        eprintln!("错误: 未指定 G代码 文件路径");
        std::process::exit(1);
    });

    if let Err(e) = process_gcode_file(&file_path, block_type) {
        eprintln!("处理 {} 文件失败: {}", file_path, e);
        std::process::exit(1);
    }

    println!("\nGcode处理完成");
}