#!/usr/bin/env python3
"""
公开专利数据批量导入工具（本地库主链路）。

用途：
1) 将公开公告数据包（CSV / JSON / JSONL）规范化为 InnoForge 专利结构；
2) 分批调用 /api/patents/import 入库；
3) 在无 CNIPR 授权场景下，快速构建可长期使用的本地检索主库。
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
import uuid
from pathlib import Path
from typing import Dict, Iterable, List
from urllib import error, request


ALIASES = {
    "patent_number": [
        "patent_number",
        "publication_number",
        "pub_number",
        "公开（公告）号",
        "公开公告号",
        "公开号",
        "申请号",
        "application_number",
    ],
    "title": ["title", "名称", "发明名称", "标题"],
    "abstract_text": ["abstract", "abstract_text", "摘要"],
    "applicant": ["applicant", "申请人", "申请（专利权）人"],
    "inventor": ["inventor", "发明人", "发明（设计）人"],
    "filing_date": ["filing_date", "申请日", "application_date"],
    "publication_date": ["publication_date", "公开日", "公开（公告）日"],
    "country": ["country", "国家", "jurisdiction"],
    "ipc_codes": ["ipc", "ipc_codes", "主分类号", "ipc分类号"],
    "cpc_codes": ["cpc", "cpc_codes", "cpc分类号"],
}


def norm_key(s: str) -> str:
    return re.sub(r"[\s_\-（）()]+", "", s.strip().lower())


def pick(row: Dict[str, object], aliases: List[str]) -> str:
    if not row:
        return ""
    indexed = {norm_key(str(k)): v for k, v in row.items()}
    for key in aliases:
        v = indexed.get(norm_key(key))
        if v is not None and str(v).strip():
            return str(v).strip()
    return ""


def norm_patent_number(pn: str) -> str:
    return re.sub(r"[\s.]+", "", (pn or "").strip().upper())


def infer_country(patent_number: str, fallback: str) -> str:
    if fallback:
        return fallback.upper()
    pn = norm_patent_number(patent_number)
    if len(pn) >= 2 and pn[:2].isalpha():
        return pn[:2]
    return "CN" if re.search(r"[\u4e00-\u9fff]", patent_number or "") else ""


def stable_id(patent_number: str, title: str, filing_date: str) -> str:
    key = norm_patent_number(patent_number)
    if not key:
        key = f"{title}|{filing_date}"
    return str(uuid.uuid5(uuid.NAMESPACE_URL, f"innoforge-public:{key}"))


def to_patent(row: Dict[str, object], source_tag: str) -> Dict[str, object]:
    pn = pick(row, ALIASES["patent_number"])
    title = pick(row, ALIASES["title"])
    if not (pn or title):
        return {}
    filing = pick(row, ALIASES["filing_date"])
    country = infer_country(pn, pick(row, ALIASES["country"]))
    return {
        "id": stable_id(pn, title, filing),
        "patent_number": pn,
        "title": title or "(无标题)",
        "abstract_text": pick(row, ALIASES["abstract_text"]),
        "description": "",
        "claims": "",
        "applicant": pick(row, ALIASES["applicant"]),
        "inventor": pick(row, ALIASES["inventor"]),
        "filing_date": filing,
        "publication_date": pick(row, ALIASES["publication_date"]),
        "grant_date": None,
        "ipc_codes": pick(row, ALIASES["ipc_codes"]),
        "cpc_codes": pick(row, ALIASES["cpc_codes"]),
        "priority_date": "",
        "country": country,
        "kind_code": "",
        "family_id": None,
        "legal_status": "",
        "citations": "[]",
        "cited_by": "[]",
        "source": source_tag,
        "raw_json": "",
        "images": "[]",
        "pdf_url": "",
    }


def read_csv(path: Path) -> Iterable[Dict[str, object]]:
    with path.open("r", encoding="utf-8-sig", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            yield row


def read_json(path: Path) -> Iterable[Dict[str, object]]:
    with path.open("r", encoding="utf-8") as f:
        obj = json.load(f)
    if isinstance(obj, list):
        for row in obj:
            if isinstance(row, dict):
                yield row
    elif isinstance(obj, dict):
        for key in ("data", "rows", "items", "results"):
            if isinstance(obj.get(key), list):
                for row in obj[key]:
                    if isinstance(row, dict):
                        yield row


def read_jsonl(path: Path) -> Iterable[Dict[str, object]]:
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(row, dict):
                yield row


def chunks(items: List[Dict[str, object]], size: int) -> Iterable[List[Dict[str, object]]]:
    for i in range(0, len(items), size):
        yield items[i : i + size]


def post_import(api_url: str, batch: List[Dict[str, object]]) -> int:
    payload = json.dumps({"patents": batch}, ensure_ascii=False).encode("utf-8")
    req = request.Request(
        api_url,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with request.urlopen(req, timeout=60) as resp:
            body = resp.read().decode("utf-8", errors="ignore")
    except error.HTTPError as e:
        detail = e.read().decode("utf-8", errors="ignore")
        raise RuntimeError(f"HTTP {e.code}: {detail[:300]}") from e
    except Exception as e:
        raise RuntimeError(f"请求失败: {e}") from e
    try:
        data = json.loads(body)
    except json.JSONDecodeError:
        raise RuntimeError(f"服务端返回非 JSON: {body[:300]}")
    return int(data.get("imported", 0))


def load_rows(path: Path, fmt: str) -> Iterable[Dict[str, object]]:
    if fmt == "csv":
        return read_csv(path)
    if fmt == "json":
        return read_json(path)
    if fmt == "jsonl":
        return read_jsonl(path)
    suffix = path.suffix.lower()
    if suffix == ".csv":
        return read_csv(path)
    if suffix in (".jsonl", ".ndjson"):
        return read_jsonl(path)
    if suffix == ".json":
        return read_json(path)
    raise ValueError(f"无法自动识别格式: {path.name}，请显式传 --format")


def main() -> int:
    ap = argparse.ArgumentParser(description="公开专利数据批量导入到 InnoForge 本地库")
    ap.add_argument("--file", required=True, help="输入文件路径（csv/json/jsonl）")
    ap.add_argument(
        "--api",
        default="http://127.0.0.1:3000/api/patents/import",
        help="导入接口地址",
    )
    ap.add_argument("--format", choices=["auto", "csv", "json", "jsonl"], default="auto")
    ap.add_argument("--batch-size", type=int, default=200)
    ap.add_argument("--source", default="public_bulk")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    path = Path(args.file)
    if not path.exists():
        print(f"[ERR] 文件不存在: {path}")
        return 2

    fmt = "auto" if args.format == "auto" else args.format
    raw_rows = list(load_rows(path, fmt))
    normalized: List[Dict[str, object]] = []
    seen = set()
    skipped = 0
    for row in raw_rows:
        pat = to_patent(row, args.source)
        if not pat:
            skipped += 1
            continue
        pid = pat["id"]
        if pid in seen:
            skipped += 1
            continue
        seen.add(pid)
        normalized.append(pat)

    if not normalized:
        print("[WARN] 没有可导入的数据（字段可能不匹配）")
        return 1

    print(
        f"[INFO] 读取 {len(raw_rows)} 行，规范化 {len(normalized)} 行，跳过 {skipped} 行，批大小 {args.batch_size}"
    )

    if args.dry_run:
        sample = normalized[0]
        print("[DRY-RUN] 首条样例:")
        print(json.dumps(sample, ensure_ascii=False, indent=2))
        return 0

    imported_total = 0
    for idx, batch in enumerate(chunks(normalized, args.batch_size), start=1):
        imported = post_import(args.api, batch)
        imported_total += imported
        print(f"[OK] batch {idx}: {imported}/{len(batch)}")

    print(
        f"[DONE] 导入完成: imported={imported_total}, normalized={len(normalized)}, skipped={skipped}, api={args.api}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
