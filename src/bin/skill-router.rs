use patent_hub::skill_router::types::RouterPaths;
use patent_hub::skill_router::SkillRouter;

fn main() {
    if let Err(error) = run() {
        let payload = serde_json::json!({
            "status": "error",
            "error": error.to_string(),
        });
        eprintln!("{}", serde_json::to_string_pretty(&payload).unwrap());
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return Ok(());
    }

    // GC command: skill-router --gc [--days N]
    if args.iter().any(|a| a == "--gc") {
        let days = args.windows(2)
            .find(|w| w[0] == "--days")
            .and_then(|w| w[1].parse::<u64>().ok())
            .unwrap_or(90);
        let workspace = std::env::current_dir()?;
        let paths = RouterPaths::for_workspace(&workspace);
        let mut registry = patent_hub::skill_router::registry::RegistryStore::load(&paths)?;
        let purged = registry.gc(days);
        registry.save(&paths)?;
        if purged.is_empty() {
            eprintln!("GC: no skills purged (threshold: {} days unused)", days);
        } else {
            eprintln!("GC: purged {} skill(s): {}", purged.len(), purged.join(", "));
        }
        return Ok(());
    }

    // Parse flags
    let mut task = String::new();
    let mut force_capability: Option<String> = None;
    let mut context_json: Option<serde_json::Value> = None;
    let mut verbose = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--capability" | "-c" => {
                i += 1;
                force_capability = args.get(i).cloned();
            }
            "--context" | "-x" => {
                i += 1;
                if let Some(raw) = args.get(i) {
                    context_json = Some(serde_json::from_str(raw).map_err(|e| {
                        anyhow::anyhow!("--context must be valid JSON: {e}")
                    })?);
                }
            }
            "--verbose" | "-v" => verbose = true,
            other => {
                if !task.is_empty() { task.push(' '); }
                task.push_str(other);
            }
        }
        i += 1;
    }

    if task.trim().is_empty() {
        anyhow::bail!("task string is required. Run with --help for usage.");
    }

    let workspace = std::env::current_dir()?;
    let paths = RouterPaths::for_workspace(&workspace);
    let router = SkillRouter::new(paths)?;

    let result = if let Some(cap) = force_capability {
        router.route_with_capability(&task, &cap, context_json)?
    } else {
        router.route_with_context(&task, context_json)?
    };

    if verbose {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Compact output: just the execution result
        let out = serde_json::json!({
            "status":     result.execution.status,
            "capability": result.capability,
            "skill":      result.skill.metadata.name,
            "lifecycle":  result.lifecycle,
            "result":     result.execution.result,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    }

    Ok(())
}

fn print_usage() {
    eprintln!("skill-router -- route a task to the best available skill\n");
    eprintln!("USAGE:");
    eprintln!("  skill-router [OPTIONS] <task>");
    eprintln!("  skill-router --gc [--days N]\n");
    eprintln!("OPTIONS:");
    eprintln!("  -c, --capability <name>   Force a specific capability (skip AI inference)");
    eprintln!("  -x, --context <json>      Pass extra context as a JSON object");
    eprintln!("  -v, --verbose             Print full RouteResult including skill metadata");
    eprintln!("  --gc                      Garbage-collect unused skills from registry");
    eprintln!("  --days N                  Days threshold for --gc (default: 90)");
    eprintln!("  -h, --help                Show this help\n");
    eprintln!("EXAMPLES:");
    eprintln!("  skill-router \"parse this yaml\"");
    eprintln!("  skill-router -c yaml_parse -x '{{\"text\":\"key: value\"}}' parse");
    eprintln!("  skill-router -v \"summarize this text\" -x '{{\"text\":\"hello world\"}}'");
    eprintln!("  skill-router --gc --days 30");
}
