import argparse
import json
import os
import re
import subprocess
import time
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any

import requests
from playwright.sync_api import sync_playwright


BASE_URL = "http://127.0.0.1:3000"
PAGES = ["/", "/search", "/idea", "/compare", "/settings", "/ai", "/patent/smoke-pat-001"]
WORKTREES = {
    "v0.5.0": r"D:\test\innoforge-v050",
    "v0.5.3": r"D:\test\innoforge-v053",
}
WORKTREE_BINS = {
    "v0.5.0": "patent-hub",
    "v0.5.3": "innoforge",
}


@dataclass
class ApiCase:
    name: str
    method: str
    url: str
    payload: Any


class StepLogger:
    def __init__(self, out_dir: Path):
        self.out_dir = out_dir
        self.steps_fp = (out_dir / "steps.jsonl").open("a", encoding="utf-8")
        self.count = 0

    def close(self):
        self.steps_fp.close()

    def log(self, step_type: str, status: str, data: dict):
        safe_data = {}
        for k, v in data.items():
            if k == "status":
                safe_data["http_status"] = v
            else:
                safe_data[k] = v
        record = {
            "ts": datetime.now().strftime("%Y-%m-%d %H:%M:%S"),
            "idx": self.count,
            "type": step_type,
            "status": status,
            **safe_data,
        }
        self.steps_fp.write(json.dumps(record, ensure_ascii=False) + "\n")
        self.steps_fp.flush()
        self.count += 1


def slug(s: str) -> str:
    s = s.strip().lower()
    s = re.sub(r"[^a-z0-9_\-]+", "_", s)
    return s[:60].strip("_") or "x"


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


def api_cases() -> list[ApiCase]:
    patent = {
        "id": "smoke-pat-001",
        "patent_number": "CNSMOKE001A",
        "title": "冒烟测试专利",
        "abstract_text": "用于测试页面按钮与接口联动",
        "description": "描述",
        "claims": "1. 一种测试方法。",
        "applicant": "Test Corp",
        "inventor": "Tester",
        "filing_date": "2026-01-01",
        "publication_date": "2026-02-01",
        "grant_date": None,
        "ipc_codes": "G06F",
        "cpc_codes": "G06F",
        "priority_date": "2026-01-01",
        "country": "CN",
        "kind_code": "A",
        "family_id": None,
        "legal_status": "pending",
        "citations": "[]",
        "cited_by": "[]",
        "source": "test",
        "raw_json": "{}",
        "created_at": "2026-04-19T00:00:00Z",
        "images": "[]",
        "pdf_url": "",
    }

    return [
        ApiCase("GET settings", "GET", "/api/settings", None),
        ApiCase("POST save serpapi", "POST", "/api/settings/serpapi", {"api_key": "A" * 24}),
        ApiCase("POST save bing", "POST", "/api/settings/bing", {"api_key": "B" * 24}),
        ApiCase("POST save lens", "POST", "/api/settings/lens", {"api_key": "L" * 12}),
        ApiCase(
            "POST save ai",
            "POST",
            "/api/settings/ai",
            {"base_url": "http://127.0.0.1:11434/v1", "api_key": "test_api_key_123", "model": "qwen2.5:7b"},
        ),
        ApiCase(
            "POST save fallbacks",
            "POST",
            "/api/settings/fallbacks",
            {
                "fallbacks": [
                    {
                        "name": "fb1",
                        "url": "https://openrouter.ai/api/v1",
                        "key": "OPENROUTER_KEY_123456",
                        "model": "google/gemini-2.0-flash-exp:free",
                    }
                ]
            },
        ),
        ApiCase("POST import patent", "POST", "/api/patents/import", {"patents": [patent]}),
        ApiCase(
            "POST search local",
            "POST",
            "/api/search",
            {"query": "测试", "page": 1, "page_size": 10, "search_type": "mixed", "sort_by": "relevance"},
        ),
        ApiCase("POST ai chat", "POST", "/api/ai/chat", {"message": "你好，请返回一句测试回复", "history": [], "web_search": False}),
        ApiCase(
            "POST ai compare",
            "POST",
            "/api/ai/compare",
            {
                "items": [
                    {"type": "text", "title": "文档A", "content": "一种散热结构"},
                    {"type": "text", "title": "文档B", "content": "一种导热通道"},
                ]
            },
        ),
        ApiCase(
            "POST ai compare-matrix",
            "POST",
            "/api/ai/compare-matrix",
            {
                "items": [
                    {"type": "text", "title": "文档A", "content": "一种散热结构"},
                    {"type": "text", "title": "文档B", "content": "一种导热通道"},
                ]
            },
        ),
        ApiCase(
            "POST ai inventiveness",
            "POST",
            "/api/ai/inventiveness-analysis",
            {
                "my_patent": {"type": "text", "title": "我的方案", "content": "一种用于手机散热的结构。"},
                "references": [{"type": "text", "title": "对比1", "content": "现有导热方案。"}],
            },
        ),
        ApiCase(
            "POST ai office-action",
            "POST",
            "/api/ai/office-action-response",
            {
                "my_patent": {"type": "text", "title": "我的方案", "content": "一种用于手机散热的结构。"},
                "office_action": {"type": "text", "content": "权利要求1不具备创造性，请陈述意见。"},
                "references": [{"type": "text", "title": "对比文献", "content": "公开了基础散热结构。"}],
            },
        ),
        ApiCase("POST idea submit", "POST", "/api/idea/submit", {"title": "实测创意", "description": "用于页面按钮回归测试", "input_type": "text"}),
    ]


def run_api(session: requests.Session, slog: StepLogger):
    results = []
    created_idea_id = None
    for case in api_cases():
        full = f"{BASE_URL}{case.url}"
        try:
            if case.method == "GET":
                resp = session.get(full, timeout=25)
            else:
                resp = session.post(full, json=case.payload, timeout=60)
            ok = 200 <= resp.status_code < 300
            body = resp.text[:280].replace("\n", " ")
            if case.name == "POST idea submit" and ok:
                try:
                    created_idea_id = resp.json().get("id")
                except Exception:
                    pass
            slog.log("api", "ok" if ok else "fail", {"name": case.name, "url": case.url, "status": resp.status_code, "body": body})
            results.append({"name": case.name, "url": case.url, "status": resp.status_code, "ok": ok, "body": body})
        except Exception as e:
            msg = str(e)[:280]
            slog.log("api", "fail", {"name": case.name, "url": case.url, "status": -1, "body": msg})
            results.append({"name": case.name, "url": case.url, "status": -1, "ok": False, "body": msg})

    if created_idea_id:
        follow = [
            ApiCase("GET idea by id", "GET", f"/api/idea/{created_idea_id}", None),
            ApiCase("GET idea report", "GET", f"/api/idea/{created_idea_id}/report", None),
            ApiCase("POST idea chat", "POST", f"/api/idea/{created_idea_id}/chat", {"message": "请输出一句测试回复", "agent": "system"}),
            ApiCase("GET idea messages", "GET", f"/api/idea/{created_idea_id}/messages", None),
            ApiCase("POST idea summarize", "POST", f"/api/idea/{created_idea_id}/summarize", {}),
            ApiCase("POST idea delete", "POST", f"/api/idea/{created_idea_id}/delete", {}),
        ]
        for case in follow:
            full = f"{BASE_URL}{case.url}"
            try:
                if case.method == "GET":
                    resp = session.get(full, timeout=30)
                else:
                    resp = session.post(full, json=case.payload, timeout=60)
                ok = 200 <= resp.status_code < 300
                body = resp.text[:280].replace("\n", " ")
                slog.log("api", "ok" if ok else "fail", {"name": case.name, "url": case.url, "status": resp.status_code, "body": body})
                results.append({"name": case.name, "url": case.url, "status": resp.status_code, "ok": ok, "body": body})
            except Exception as e:
                msg = str(e)[:280]
                slog.log("api", "fail", {"name": case.name, "url": case.url, "status": -1, "body": msg})
                results.append({"name": case.name, "url": case.url, "status": -1, "ok": False, "body": msg})

    return results


def shot(page, shots_dir: Path, idx: int, name: str) -> str:
    path = shots_dir / f"{idx:04d}_{slug(name)}.png"
    page.screenshot(path=str(path), full_page=True)
    return str(path.name)


def close_transient_overlays(page, slog: StepLogger, version: str, route: str):
    closed = []
    try:
        paste_modal = page.locator("#paste-modal")
        if paste_modal.count() > 0 and paste_modal.first.is_visible():
            cancel_btn = page.locator("#paste-modal .btn-modal-cancel")
            if cancel_btn.count() > 0 and cancel_btn.first.is_visible():
                cancel_btn.first.click(timeout=1200)
            else:
                page.keyboard.press("Escape")
            page.wait_for_timeout(150)
            closed.append("paste-modal")
    except Exception:
        pass
    try:
        pdf_modal = page.locator("#pdf-preview-modal")
        if pdf_modal.count() > 0 and pdf_modal.first.is_visible():
            close_btn = page.locator("#pdf-preview-modal .pdf-modal-close")
            if close_btn.count() > 0 and close_btn.first.is_visible():
                close_btn.first.click(timeout=1200)
            else:
                page.keyboard.press("Escape")
            page.wait_for_timeout(150)
            closed.append("pdf-preview-modal")
    except Exception:
        pass
    if closed:
        slog.log("ui-modal", "ok", {"version": version, "route": route, "closed": closed})
    return closed


def run_ui(version: str, out_dir: Path, slog: StepLogger):
    page_results = []
    console_errors = []
    shots_dir = out_dir / "screenshots"
    videos_dir = out_dir / "videos"
    shots_dir.mkdir(parents=True, exist_ok=True)
    videos_dir.mkdir(parents=True, exist_ok=True)
    shot_idx = 0

    with sync_playwright() as p:
        browser = p.chromium.launch(channel="msedge", headless=True)
        context = browser.new_context(viewport={"width": 1440, "height": 900}, record_video_dir=str(videos_dir))
        page = context.new_page()
        page.set_default_timeout(5000)
        page.on("dialog", lambda d: d.accept())
        page.on("filechooser", lambda fc: fc.set_files([]))
        page.on("pageerror", lambda e: console_errors.append(str(e)))
        page.on("console", lambda msg: console_errors.append(msg.text) if msg.type == "error" else None)

        for route in PAGES:
            info = {
                "route": route,
                "http_ok": False,
                "input_total": 0,
                "input_filled": 0,
                "textarea_total": 0,
                "textarea_filled": 0,
                "select_total": 0,
                "select_set": 0,
                "button_total": 0,
                "button_ok": 0,
                "button_fail": 0,
                "button_skip": 0,
                "buttons": [],
            }
            try:
                resp = page.goto(f"{BASE_URL}{route}", wait_until="domcontentloaded", timeout=30000)
                info["http_ok"] = resp is not None and 200 <= resp.status < 400
            except Exception as e:
                slog.log("ui-page", "fail", {"version": version, "route": route, "error": str(e)})
                page_results.append(info)
                continue

            page.wait_for_timeout(700)
            close_transient_overlays(page, slog, version, route)
            shot_name = shot(page, shots_dir, shot_idx, f"{version}_{route}_loaded")
            shot_idx += 1
            slog.log("ui-page", "ok", {"version": version, "route": route, "screenshot": shot_name})

            inputs = page.locator("input")
            info["input_total"] = inputs.count()
            for i in range(info["input_total"]):
                inp = inputs.nth(i)
                try:
                    typ = (inp.get_attribute("type") or "text").lower()
                    if typ in {"hidden", "file", "checkbox", "radio"}:
                        continue
                    if not inp.is_visible() or not inp.is_enabled():
                        continue
                    val = "2026-04-19" if typ == "date" else "smoke-input"
                    inp.fill(val)
                    info["input_filled"] += 1
                    slog.log("ui-input", "ok", {"version": version, "route": route, "index": i, "type": typ, "value": val})
                except Exception as e:
                    slog.log("ui-input", "fail", {"version": version, "route": route, "index": i, "error": str(e)[:180]})

            textareas = page.locator("textarea")
            info["textarea_total"] = textareas.count()
            for i in range(info["textarea_total"]):
                ta = textareas.nth(i)
                try:
                    if not ta.is_visible() or not ta.is_enabled():
                        continue
                    ta.fill("smoke-textarea")
                    info["textarea_filled"] += 1
                    slog.log("ui-textarea", "ok", {"version": version, "route": route, "index": i})
                except Exception as e:
                    slog.log("ui-textarea", "fail", {"version": version, "route": route, "index": i, "error": str(e)[:180]})

            selects = page.locator("select")
            info["select_total"] = selects.count()
            for i in range(info["select_total"]):
                sel = selects.nth(i)
                try:
                    if not sel.is_visible() or not sel.is_enabled():
                        continue
                    options = sel.locator("option")
                    if options.count() == 0:
                        continue
                    v = options.nth(0).get_attribute("value")
                    if v is not None:
                        sel.select_option(value=v)
                        info["select_set"] += 1
                        slog.log("ui-select", "ok", {"version": version, "route": route, "index": i, "value": v})
                except Exception as e:
                    slog.log("ui-select", "fail", {"version": version, "route": route, "index": i, "error": str(e)[:180]})

            buttons = page.locator("button")
            info["button_total"] = buttons.count()
            for i in range(info["button_total"]):
                row = {"index": i, "text": "", "ok": False, "error": "", "shot_before": "", "shot_after": ""}
                try:
                    current_count = page.locator("button").count()
                    if i >= current_count:
                        row["error"] = "dynamic_missing"
                        info["button_skip"] += 1
                        info["buttons"].append(row)
                        slog.log("ui-button", "skip", {"version": version, "route": route, "index": i, "reason": row["error"]})
                        continue
                    btn = page.locator("button").nth(i)
                    txt = (btn.inner_text() or "").strip().replace("\n", " ")
                    row["text"] = txt[:120]
                    if not btn.is_visible() or not btn.is_enabled():
                        row["error"] = "not_visible_or_disabled"
                        info["button_skip"] += 1
                        info["buttons"].append(row)
                        slog.log("ui-button", "skip", {"version": version, "route": route, "index": i, "text": row["text"], "reason": row["error"]})
                        continue
                    close_transient_overlays(page, slog, version, route)
                    row["shot_before"] = shot(page, shots_dir, shot_idx, f"{version}_{route}_btn_{i}_before")
                    shot_idx += 1
                    btn.click(timeout=4000, no_wait_after=True)
                    page.wait_for_timeout(500)
                    close_transient_overlays(page, slog, version, route)
                    row["shot_after"] = shot(page, shots_dir, shot_idx, f"{version}_{route}_btn_{i}_after")
                    shot_idx += 1
                    row["ok"] = True
                    info["button_ok"] += 1
                    slog.log("ui-button", "ok", {"version": version, "route": route, "index": i, "text": row["text"], "before": row["shot_before"], "after": row["shot_after"]})
                except Exception as e:
                    first_error = str(e)[:220]
                    close_transient_overlays(page, slog, version, route)
                    try:
                        btn.click(timeout=2500, no_wait_after=True)
                        page.wait_for_timeout(350)
                        close_transient_overlays(page, slog, version, route)
                        row["shot_after"] = shot(page, shots_dir, shot_idx, f"{version}_{route}_btn_{i}_after_retry")
                        shot_idx += 1
                        row["ok"] = True
                        row["error"] = f"retry_after_error:{first_error}"
                        info["button_ok"] += 1
                        slog.log("ui-button", "ok", {"version": version, "route": route, "index": i, "text": row["text"], "after": row["shot_after"], "note": "retry_success"})
                    except Exception as e2:
                        row["error"] = str(e2)[:220]
                        info["button_fail"] += 1
                        slog.log("ui-button", "fail", {"version": version, "route": route, "index": i, "text": row["text"], "error": row["error"], "first_error": first_error})
                info["buttons"].append(row)

            page_results.append(info)

        context.close()
        browser.close()

    return page_results, console_errors


def write_markdown(out_dir: Path, version: str, api: list[dict], pages: list[dict], console_errors: list[str]):
    md = out_dir / "summary.md"
    api_pass = sum(1 for x in api if x["ok"])
    api_fail = len(api) - api_pass
    btn_total = sum(p["button_total"] for p in pages)
    btn_ok = sum(p["button_ok"] for p in pages)
    btn_fail = sum(p["button_fail"] for p in pages)
    btn_skip = sum(p.get("button_skip", 0) for p in pages)

    lines = []
    lines.append(f"# 可见化实测结果 - {version}")
    lines.append("")
    lines.append(f"- API 通过：{api_pass}/{len(api)}")
    lines.append(f"- 按钮成功：{btn_ok}/{btn_total}")
    lines.append(f"- 按钮失败：{btn_fail}")
    lines.append(f"- 按钮跳过：{btn_skip}")
    lines.append(f"- 控制台错误数：{len(console_errors)}")
    lines.append("")
    lines.append("## 页面摘要")
    lines.append("| 页面 | 按钮成功/失败/跳过/总数 | 输入填充 | 文本框填充 | 下拉设置 |")
    lines.append("|---|---:|---:|---:|---:|")
    for p in pages:
        lines.append(
            f"| `{p['route']}` | {p['button_ok']}/{p['button_fail']}/{p.get('button_skip',0)}/{p['button_total']} | {p['input_filled']}/{p['input_total']} | {p['textarea_filled']}/{p['textarea_total']} | {p['select_set']}/{p['select_total']} |"
        )
    lines.append("")
    lines.append("## API 详情")
    for c in api:
        lines.append(f"- [{'PASS' if c['ok'] else 'FAIL'}] `{c['name']}` `{c['status']}` `{c['url']}`")
    lines.append("")
    lines.append("## 控制台错误（前10条）")
    for e in console_errors[:10]:
        lines.append(f"- {e}")

    md.write_text("\n".join(lines), encoding="utf-8")


def run_version(version: str, worktree: str, run_root: Path):
    version_dir = run_root / version
    version_dir.mkdir(parents=True, exist_ok=True)
    slog = StepLogger(version_dir)
    server_log = (version_dir / "server.log").open("w", encoding="utf-8")

    bin_name = WORKTREE_BINS.get(version)
    cmd = ["cargo", "run", "--quiet"]
    if bin_name:
        cmd.extend(["--bin", bin_name])

    proc = subprocess.Popen(
        cmd,
        cwd=worktree,
        stdout=server_log,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if not wait_server(proc, timeout_sec=260):
        slog.log("boot", "fail", {"version": version, "message": "server_start_failed"})
        stop_server(proc)
        server_log.close()
        slog.close()
        return {"version": version, "boot_ok": False}

    slog.log("boot", "ok", {"version": version, "message": "server_ready"})
    session = requests.Session()
    try:
        api = run_api(session, slog)
        pages, console_errors = run_ui(version, version_dir, slog)
        write_markdown(version_dir, version, api, pages, console_errors)
        result = {"version": version, "boot_ok": True, "api": api, "pages": pages, "console_errors": console_errors}
        (version_dir / "result.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        return result
    finally:
        stop_server(proc)
        server_log.close()
        slog.close()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True, help="输出目录")
    ap.add_argument("--versions", default="v0.5.0,v0.5.3", help="逗号分隔版本")
    args = ap.parse_args()

    out_root = Path(args.out)
    out_root.mkdir(parents=True, exist_ok=True)

    versions = [v.strip() for v in args.versions.split(",") if v.strip()]
    all_results = []
    for v in versions:
        wt = WORKTREES.get(v)
        if not wt or not Path(wt).exists():
            all_results.append({"version": v, "boot_ok": False, "error": "worktree_missing"})
            continue
        print(f"== running {v} ==")
        all_results.append(run_version(v, wt, out_root))

    final_path = out_root / "all_results.json"
    final_path.write_text(json.dumps(all_results, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"saved: {final_path}")


if __name__ == "__main__":
    main()
