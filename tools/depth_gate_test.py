import argparse
import json
import re
import subprocess
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

import requests


BASE_URL = "http://127.0.0.1:3000"
DEFAULT_WORKTREE = r"D:\test\innoforge-v053"
DEFAULT_BIN = "innoforge"


@dataclass
class DepthCase:
    name: str
    url: str
    payload: Any
    response_key: str  # "content" | "analysis"


def wait_server(proc: subprocess.Popen, timeout_sec: int = 240) -> bool:
    start = time.time()
    while time.time() - start < timeout_sec:
        if proc.poll() is not None:
            return False
        try:
            r = requests.get(f"{BASE_URL}/", timeout=1)
            if r.status_code == 200:
                return True
        except Exception:
            pass
        time.sleep(1)
    return False


def stop_server(proc: subprocess.Popen) -> None:
    if proc.poll() is not None:
        return
    try:
        proc.terminate()
        proc.wait(timeout=15)
    except Exception:
        try:
            proc.kill()
        except Exception:
            pass


def make_cases() -> list[DepthCase]:
    return [
        DepthCase(
            name="ai_chat_depth",
            url="/api/ai/chat",
            payload={
                "message": (
                    "请围绕“电动车热管理系统”给出研发分析。"
                    "必须包含：结论、至少5条依据、至少3条风险、至少3条可执行建议、关键边界条件。"
                ),
                "history": [],
                "web_search": False,
            },
            response_key="content",
        ),
        DepthCase(
            name="ai_compare_depth",
            url="/api/ai/compare",
            payload={
                "items": [
                    {
                        "type": "text",
                        "title": "方案A",
                        "content": (
                            "一种电池包液冷结构，包括蛇形流道、分区温控阀、"
                            "热失控隔离层，目标是将温差控制在2摄氏度以内。"
                        ),
                    },
                    {
                        "type": "text",
                        "title": "方案B",
                        "content": (
                            "一种电池包相变+风冷混合散热结构，采用相变材料夹层和双通道风道，"
                            "强调低成本与维护便利。"
                        ),
                    },
                ]
            },
            response_key="content",
        ),
        DepthCase(
            name="inventiveness_depth",
            url="/api/ai/inventiveness-analysis",
            payload={
                "my_patent": {
                    "type": "text",
                    "title": "我的方案",
                    "content": (
                        "一种用于动力电池包的多层热扩散结构，结合液冷板与可控相变材料，"
                        "在高倍率工况下维持温差小于3摄氏度，并在热失控时进行分区隔离。"
                    ),
                },
                "references": [
                    {
                        "type": "text",
                        "title": "对比文献1",
                        "content": "公开了基础液冷板结构，但未涉及可控相变材料。",
                    },
                    {
                        "type": "text",
                        "title": "对比文献2",
                        "content": "公开了热失控隔离设计，但温控精度不足。",
                    },
                ],
            },
            response_key="analysis",
        ),
        DepthCase(
            name="office_action_depth",
            url="/api/ai/office-action-response",
            payload={
                "my_patent": {
                    "type": "text",
                    "title": "我的方案",
                    "content": (
                        "一种用于动力电池包的多层热扩散结构，结合液冷板与可控相变材料，"
                        "在高倍率工况下维持温差小于3摄氏度，并在热失控时进行分区隔离。"
                    ),
                },
                "office_action": {
                    "type": "text",
                    "content": (
                        "审查意见：权利要求1相对于对比文献D1、D2不具备创造性，"
                        "请申请人陈述区别技术特征及其产生的技术效果。"
                    ),
                },
                "references": [
                    {
                        "type": "text",
                        "title": "D1",
                        "content": "公开液冷结构和常规导热层。",
                    },
                    {
                        "type": "text",
                        "title": "D2",
                        "content": "公开热失控隔离结构。",
                    },
                ],
            },
            response_key="analysis",
        ),
    ]


def normalize_text(v: Any) -> str:
    if isinstance(v, str):
        return v
    if isinstance(v, (dict, list)):
        return json.dumps(v, ensure_ascii=False)
    return str(v)


def evaluate_depth(text: str) -> dict[str, Any]:
    t = text.strip()
    lower = t.lower()

    hard_fail_patterns = [
        "ai error",
        "ai 错误",
        "api key",
        "无效或已过期",
        "请到「设置」页面",
        "分析失败",
        "error",
        "失败",
        "未配置",
        "requires more credits",
        "fewer max_tokens",
        "upgrade to a paid account",
        "credits",
        "额度",
    ]
    hard_fail_hit = [p for p in hard_fail_patterns if p in lower or p in t]

    lines = [x.strip() for x in re.split(r"[\r\n]+", t) if x.strip()]
    bullet_lines = sum(
        1
        for x in lines
        if re.match(r"^(\d+[\.、\)]|[-*•])\s*", x)
    )

    patent_refs = re.findall(r"\b[A-Z]{2}\d{6,}[A-Z]?\d*\b", t)
    evidence_keywords = [
        "依据",
        "证据",
        "专利号",
        "权利要求",
        "对比文献",
        "摘要",
        "申请人",
        "技术效果",
        "相似度",
    ]
    evidence_hits = sum(t.count(k) for k in evidence_keywords) + len(patent_refs)

    dim_keywords = [
        "技术领域",
        "技术问题",
        "技术方案",
        "创新点",
        "保护范围",
        "侵权风险",
        "区别特征",
        "实施路径",
        "边界条件",
        "结论",
        "建议",
    ]
    dim_hit_set = {k for k in dim_keywords if k in t}

    risk_keywords = ["风险", "不确定", "边界", "局限", "代价", "缺陷", "副作用", "风险点"]
    risk_hits = sum(t.count(k) for k in risk_keywords)

    suggestion_keywords = ["建议", "可执行", "下一步", "优先", "实施", "路线", "动作"]
    suggestion_hits = sum(t.count(k) for k in suggestion_keywords)

    conclusion_keywords = ["结论", "综上", "因此", "判定", "建议结论"]
    conclusion_hits = sum(t.count(k) for k in conclusion_keywords)

    metrics = {
        "chars": len(t),
        "lines": len(lines),
        "bullet_lines": bullet_lines,
        "evidence_hits": evidence_hits,
        "dimension_hits": len(dim_hit_set),
        "risk_hits": risk_hits,
        "suggestion_hits": suggestion_hits,
        "conclusion_hits": conclusion_hits,
        "hard_fail_hit": hard_fail_hit,
    }

    thresholds = {
        "chars": 350,
        "evidence_hits": 5,
        "dimension_hits": 3,
        "risk_hits": 3,
        "suggestion_hits": 3,
        "conclusion_hits": 1,
    }

    checks = {
        "chars_ok": metrics["chars"] >= thresholds["chars"],
        "evidence_ok": metrics["evidence_hits"] >= thresholds["evidence_hits"],
        "dimension_ok": metrics["dimension_hits"] >= thresholds["dimension_hits"],
        "risk_ok": metrics["risk_hits"] >= thresholds["risk_hits"],
        "suggestion_ok": metrics["suggestion_hits"] >= thresholds["suggestion_hits"],
        "conclusion_ok": metrics["conclusion_hits"] >= thresholds["conclusion_hits"],
        "hard_fail_ok": len(hard_fail_hit) == 0,
    }

    passed = all(checks.values())
    return {
        "passed": passed,
        "metrics": metrics,
        "thresholds": thresholds,
        "checks": checks,
    }


def run_depth_cases() -> list[dict[str, Any]]:
    out = []
    session = requests.Session()
    for case in make_cases():
        full = f"{BASE_URL}{case.url}"
        rec: dict[str, Any] = {
            "name": case.name,
            "url": case.url,
            "status": -1,
            "ok": False,
            "response_text": "",
            "eval": None,
        }
        try:
            r = session.post(full, json=case.payload, timeout=120)
            rec["status"] = r.status_code
            rec["ok"] = 200 <= r.status_code < 300
            try:
                body = r.json()
            except Exception:
                body = {"raw": r.text}

            if case.response_key in body:
                txt = normalize_text(body[case.response_key])
            elif "error" in body:
                txt = normalize_text(body["error"])
            elif "raw" in body:
                txt = normalize_text(body["raw"])
            else:
                txt = normalize_text(body)
            rec["response_text"] = txt
            rec["eval"] = evaluate_depth(txt)
        except Exception as e:
            rec["response_text"] = str(e)
            rec["eval"] = evaluate_depth(rec["response_text"])
        out.append(rec)
    return out


def write_report(out_dir: Path, worktree: str, bin_name: str, cases: list[dict[str, Any]]) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / "depth_gate_result.json"
    md_path = out_dir / "depth_gate_summary.md"

    passed = sum(1 for c in cases if c.get("eval", {}).get("passed"))
    total = len(cases)
    gate_pass = passed == total

    result = {
        "ts": datetime.now().isoformat(),
        "base_url": BASE_URL,
        "worktree": worktree,
        "bin": bin_name,
        "case_passed": passed,
        "case_total": total,
        "gate_pass": gate_pass,
        "cases": cases,
    }
    json_path.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")

    lines = [
        "# 深度门禁测试结果",
        "",
        f"- 时间：{result['ts']}",
        f"- 工作树：`{worktree}`",
        f"- 二进制：`{bin_name}`",
        f"- 用例通过：`{passed}/{total}`",
        f"- 门禁结论：`{'通过' if gate_pass else '失败'}`",
        "",
        "## 用例详情",
        "| 用例 | HTTP | 门禁 | 长度 | 依据 | 维度 | 风险 | 建议 | 结论 |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]

    for c in cases:
        ev = c.get("eval", {})
        m = ev.get("metrics", {})
        lines.append(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} |".format(
                c["name"],
                c["status"],
                "PASS" if ev.get("passed") else "FAIL",
                m.get("chars", 0),
                m.get("evidence_hits", 0),
                m.get("dimension_hits", 0),
                m.get("risk_hits", 0),
                m.get("suggestion_hits", 0),
                m.get("conclusion_hits", 0),
            )
        )

    lines += ["", "## 失败原因（自动提取）"]
    for c in cases:
        ev = c.get("eval", {})
        if ev.get("passed"):
            continue
        checks = ev.get("checks", {})
        fail_keys = [k for k, v in checks.items() if not v]
        hard = ev.get("metrics", {}).get("hard_fail_hit", [])
        lines.append(f"- `{c['name']}`: fail_checks={fail_keys} hard_fail={hard}")

    lines += ["", "## 原始输出摘录（前 600 字）"]
    for c in cases:
        excerpt = c.get("response_text", "")[:600].replace("\n", " ")
        lines.append(f"- `{c['name']}`: {excerpt}")

    md_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worktree", default=DEFAULT_WORKTREE, help="待测工作树目录")
    parser.add_argument("--bin", default=DEFAULT_BIN, help="cargo run --bin 名称")
    parser.add_argument("--out", required=True, help="报告输出目录")
    args = parser.parse_args()

    out_dir = Path(args.out)
    server_log = out_dir / "server.log"
    out_dir.mkdir(parents=True, exist_ok=True)

    proc = subprocess.Popen(
        ["cargo", "run", "--quiet", "--bin", args.bin],
        cwd=args.worktree,
        stdout=server_log.open("w", encoding="utf-8"),
        stderr=subprocess.STDOUT,
        text=True,
    )
    if not wait_server(proc):
        write_report(
            out_dir,
            args.worktree,
            args.bin,
            [
                {
                    "name": "boot",
                    "url": "/",
                    "status": -1,
                    "ok": False,
                    "response_text": "server_start_failed",
                    "eval": evaluate_depth("server_start_failed"),
                }
            ],
        )
        stop_server(proc)
        raise SystemExit(2)

    try:
        cases = run_depth_cases()
        write_report(out_dir, args.worktree, args.bin, cases)
    finally:
        stop_server(proc)


if __name__ == "__main__":
    main()
