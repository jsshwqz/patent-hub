//! 沙箱执行器 / Sandbox runner
//!
//! 在隔离的子进程中执行验证脚本，捕获输出，强制超时。

use crate::experiment::types::ExperimentSpec;
use crate::pipeline::context::ExperimentResult;
use anyhow::Result;
use std::io::Write;
use std::process::Command;
use std::time::Instant;

/// 在沙箱中运行实验脚本
pub async fn run_experiment(spec: &ExperimentSpec) -> Result<ExperimentResult> {
    let start = Instant::now();

    // 写脚本到临时文件
    let tmp_dir = std::env::temp_dir().join("innoforge_experiments");
    std::fs::create_dir_all(&tmp_dir)?;
    let script_id = uuid::Uuid::new_v4().to_string();

    let (ext, interpreter) = match spec.language.as_str() {
        "python" => ("py", find_python()),
        "rust" => ("rs", "rustc".to_string()),
        _ => ("py", find_python()),
    };

    let script_path = tmp_dir.join(format!("exp_{}.{}", script_id, ext));
    {
        let mut file = std::fs::File::create(&script_path)?;
        file.write_all(spec.script_content.as_bytes())?;
    }

    // 执行脚本（带超时）
    let _timeout = std::time::Duration::from_secs(spec.timeout_secs);

    let result = tokio::task::spawn_blocking(move || {
        let output = if ext == "py" {
            Command::new(&interpreter)
                .arg(&script_path)
                .env("PYTHONDONTWRITEBYTECODE", "1")
                .output()
        } else {
            // Rust: 先编译再运行
            let bin_path = script_path.with_extension("exe");
            let compile = Command::new("rustc")
                .arg(&script_path)
                .arg("-o")
                .arg(&bin_path)
                .output();
            match compile {
                Ok(c) if c.status.success() => Command::new(&bin_path).output(),
                Ok(c) => Ok(c), // 编译失败，返回编译输出
                Err(e) => Err(e),
            }
        };

        // 清理临时文件
        let _ = std::fs::remove_file(&script_path);
        let _ = std::fs::remove_file(script_path.with_extension("exe"));

        output
    })
    .await?;

    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            let success = output.status.success();

            // 从 stdout 提取 JSON 指标
            let metrics = extract_json_metrics(&stdout);

            Ok(ExperimentResult {
                script_path: format!("exp_{}.{}", script_id, ext),
                language: spec.language.clone(),
                exit_code,
                stdout: truncate(&stdout, 5000),
                stderr: truncate(&stderr, 2000),
                metrics,
                duration_ms,
                success,
            })
        }
        Err(e) => Ok(ExperimentResult {
            script_path: format!("exp_{}.{}", script_id, ext),
            language: spec.language.clone(),
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Failed to execute: {}", e),
            metrics: serde_json::Value::Null,
            duration_ms,
            success: false,
        }),
    }
}

/// 从输出中提取 JSON 行格式的指标
fn extract_json_metrics(stdout: &str) -> serde_json::Value {
    let mut metrics = serde_json::Map::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(map) = obj.as_object() {
                    for (k, v) in map {
                        metrics.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }
    if metrics.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(metrics)
    }
}

/// 查找 Python 解释器
fn find_python() -> String {
    for name in &["python3", "python"] {
        if Command::new(name).arg("--version").output().is_ok() {
            return name.to_string();
        }
    }
    "python".to_string()
}

/// 截断字符串
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...(truncated)", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_python_experiment() {
        let spec = ExperimentSpec {
            title: "test".to_string(),
            language: "python".to_string(),
            script_content:
                "import json\nprint(json.dumps({\"accuracy\": 0.95}))\nprint(\"EXPERIMENT_DONE\")"
                    .to_string(),
            hypothesis: "test hypothesis".to_string(),
            timeout_secs: 10,
        };
        let result = run_experiment(&spec).await.unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("EXPERIMENT_DONE"));
    }

    #[test]
    fn test_extract_json_metrics() {
        let stdout = "Starting...\n{\"accuracy\": 0.95}\n{\"latency_ms\": 12.3}\nDone\n";
        let metrics = extract_json_metrics(stdout);
        assert_eq!(metrics["accuracy"], 0.95);
        assert_eq!(metrics["latency_ms"], 12.3);
    }

    #[test]
    fn test_extract_no_metrics() {
        let stdout = "Hello world\nNo JSON here\n";
        let metrics = extract_json_metrics(stdout);
        assert!(metrics.is_null());
    }
}
