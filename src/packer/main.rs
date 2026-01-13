//! WinClean Rules Packer
//! 将YAML规则打包为二进制格式的工具

use anyhow::Result;
use clap::{Parser, Subcommand};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 命令行参数
#[derive(Parser, Debug)]
#[command(name = "winclean-rules-packer")]
#[command(author = "WinClean Contributors")]
#[command(version = "0.1.0")]
#[command(about = "WinClean Rules Packer - 将YAML规则打包为二进制格式", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 打包规则
    Pack {
        /// 输入目录（YAML规则所在目录）
        #[arg(short, long, default_value = "./rules")]
        input: PathBuf,

        /// 输出文件路径
        #[arg(short, long, default_value = "./dist/rules.bin")]
        output: PathBuf,

        /// 压缩算法: none, zstd
        #[arg(short, long, default_value = "zstd")]
        compress: String,
    },

    /// 解包规则
    Unpack {
        /// 输入文件路径（二进制规则包）
        #[arg(short, long)]
        input: PathBuf,

        /// 输出目录
        #[arg(short, long, default_value = "./rules_unpacked")]
        output: PathBuf,
    },

    /// 显示规则包信息
    Info {
        /// 输入文件路径（二进制规则包）
        #[arg(short, long)]
        input: PathBuf,
    },
}

/// 规则元数据
#[derive(Serialize, Deserialize, Debug, Clone)]
struct RuleMetadata {
    id: String,
    name: String,
    risk: String,
    systeminfo: Vec<String>,
    update: String,
    author: Option<String>,
    description: Option<String>,
    category: String,
    filename: String,
}

/// 规则包头信息
#[derive(Serialize, Deserialize, Debug)]
struct RulesPackageHeader {
    version: u32,
    created_at: u64,
    rule_count: usize,
    compression: String,
    categories: Vec<String>,
}

/// 规则包结构
#[derive(Serialize, Deserialize, Debug)]
struct RulesPackage {
    header: RulesPackageHeader,
    rules: Vec<SerializedRule>,
}

/// 序列化后的规则
#[derive(Serialize, Deserialize, Debug)]
struct SerializedRule {
    metadata: RuleMetadata,
    yaml_content: String,
    paths: Vec<String>,
    registry_entries: Vec<RegistryEntry>,
}

/// 注册表条目
#[derive(Serialize, Deserialize, Debug)]
struct RegistryEntry {
    path: String,
    key: String,
    value: Option<String>,
    value_data: Option<String>,
    action: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Pack { input, output, compress } => {
            pack_rules(&input, &output, &compress)
        }
        Commands::Unpack { input, output } => {
            unpack_rules(&input, &output)
        }
        Commands::Info { input } => {
            show_info(&input)
        }
    }
}

/// 打包规则
fn pack_rules(input: &PathBuf, output: &PathBuf, compress: &str) -> Result<()> {
    println!("打包规则: {:?}", input);

    // 创建输出目录
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    // 收集所有规则文件
    let mut rules = Vec::new();
    let mut categories = Vec::new();

    let pattern = format!("{}/*/*.yaml", input.display());
    for entry in glob(&pattern)? {
        let path = entry?;
        if path.is_file() {
            println!("  处理: {:?}", path);

            // 读取并解析YAML
            let content = fs::read_to_string(&path)?;
            let rule: serde_yaml::Value = serde_yaml::from_str(&content)?;

            // 提取元数据
            let metadata = extract_metadata(&path, &rule)?;
            let (paths, registry) = extract_matches(&rule)?;

            // 序列化规则
            let serialized = SerializedRule {
                metadata: metadata.clone(),
                yaml_content: content,
                paths,
                registry_entries: registry,
            };

            rules.push(serialized);

            // 记录分类
            if !categories.contains(&metadata.category) {
                categories.push(metadata.category);
            }
        }
    }

    // 创建包头
    let header = RulesPackageHeader {
        version: 1,
        created_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs(),
        rule_count: rules.len(),
        compression: compress.to_string(),
        categories,
    };

    // 创建包
    let package = RulesPackage {
        header,
        rules,
    };

    // 序列化
    let serialized = bincode::serialize(&package)?;
    let original_size = serialized.len();

    // 压缩
    let compressed = if compress == "zstd" {
        let mut encoder = zstd::stream::Encoder::new(Vec::new(), 0)?;
        encoder.write_all(&serialized)?;
        encoder.finish()?
    } else if compress == "none" {
        serialized
    } else {
        anyhow::bail!("不支持的压缩算法: {}", compress);
    };

    // 写入输出文件
    fs::write(output, &compressed)?;
    println!("已生成规则包: {:?}", output);
    println!("规则数量: {}", package.header.rule_count);
    println!("压缩前大小: {} bytes", original_size);
    println!("压缩后大小: {} bytes", compressed.len());

    Ok(())
}

/// 解包规则
fn unpack_rules(input: &PathBuf, output: &PathBuf) -> Result<()> {
    println!("解包规则: {:?}", input);

    // 读取文件
    let compressed = fs::read(input)?;

    // 解压
    let decompressed = if let Ok(reader) = zstd::stream::Decoder::new(&compressed[..]) {
        let mut decoder = reader;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        compressed.clone()
    };

    // 反序列化
    let package: RulesPackage = bincode::deserialize(&decompressed)?;

    // 创建输出目录
    fs::create_dir_all(output)?;

    // 写入规则文件
    for rule in &package.rules {
        let category_dir = output.join(&rule.metadata.category);
        fs::create_dir_all(&category_dir)?;

        let output_path = category_dir.join(&rule.metadata.filename);
        fs::write(&output_path, &rule.yaml_content)?;
        println!("  提取: {:?}", output_path);
    }

    println!("已解包到: {:?}", output);
    println!("规则数量: {}", package.header.rule_count);

    Ok(())
}

/// 显示规则包信息
fn show_info(input: &PathBuf) -> Result<()> {
    println!("规则包信息: {:?}", input);

    // 读取文件
    let compressed = fs::read(input)?;

    // 解压
    let decompressed = if let Ok(reader) = zstd::stream::Decoder::new(&compressed[..]) {
        let mut decoder = reader;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        compressed.clone()
    };

    // 反序列化
    let package: RulesPackage = bincode::deserialize(&decompressed)?;

    println!("版本: {}", package.header.version);
    println!("创建时间: {}", package.header.created_at);
    println!("规则数量: {}", package.header.rule_count);
    println!("压缩算法: {}", package.header.compression);
    println!("分类: {:?}", package.header.categories);
    println!("大小: {} bytes", compressed.len());

    println!("\n规则列表:");
    for rule in &package.rules {
        println!("  - [{}] {} (风险: {})", rule.metadata.id, rule.metadata.name, rule.metadata.risk);
    }

    Ok(())
}

/// 从YAML中提取元数据
fn extract_metadata(path: &PathBuf, rule: &serde_yaml::Value) -> Result<RuleMetadata> {
    let category = path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("other")
        .to_string();

    Ok(RuleMetadata {
        id: rule["id"].as_str().unwrap_or("").to_string(),
        name: rule["name"].as_str().unwrap_or("").to_string(),
        risk: rule["risk"].as_str().unwrap_or("low").to_string(),
        systeminfo: rule["systeminfo"].as_sequence()
            .map(|s| s.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default(),
        update: rule["update"].as_str().unwrap_or("").to_string(),
        author: rule["author"].as_str().map(|s| s.to_string()),
        description: rule["description"].as_str().map(|s| s.to_string()),
        category,
        filename: path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.yaml")
            .to_string(),
    })
}

/// 从YAML中提取匹配规则
fn extract_matches(rule: &serde_yaml::Value) -> Result<(Vec<String>, Vec<RegistryEntry>)> {
    let mut paths = Vec::new();
    let mut registry = Vec::new();

    if let Some(match_section) = rule.get("match") {
        if let Some(paths_section) = match_section.get("path") {
            if let Some(paths_array) = paths_section.as_sequence() {
                for p in paths_array {
                    if let Some(s) = p.as_str() {
                        paths.push(s.to_string());
                    }
                }
            }
        }

        if let Some(registry_section) = match_section.get("registry") {
            if let Some(registry_array) = registry_section.as_sequence() {
                for r in registry_array {
                    let entry = RegistryEntry {
                        path: r["path"].as_str().unwrap_or("").to_string(),
                        key: r["key"].as_str().unwrap_or("*").to_string(),
                        value: r["value"].as_str().map(|s| s.to_string()),
                        value_data: r["value_data"].as_str().map(|s| s.to_string()),
                        action: r["action"].as_str().unwrap_or("delete_key").to_string(),
                    };
                    registry.push(entry);
                }
            }
        }
    }

    Ok((paths, registry))
}
