# 公开数据本地化导入（无 CNIPR 授权可长期运行）

当无法获取或维持 CNIPR 授权时，推荐把公开公告数据包导入本地库，作为主检索链路。

## 1. 准备数据文件

支持格式：
- `CSV`
- `JSON`（数组，或对象下的 `data/rows/items/results`）
- `JSONL/NDJSON`

常见字段（中英都支持，缺失字段会自动留空）：
- `patent_number / 公开（公告）号 / 公开号 / 申请号`
- `title / 名称 / 发明名称`
- `abstract / abstract_text / 摘要`
- `applicant / 申请人`
- `inventor / 发明人`
- `filing_date / 申请日`

## 2. 启动应用

```powershell
cargo run --release --bin innoforge
```

## 3. 先做 dry-run 检查映射

```powershell
python tools/import_public_patents.py --file D:\data\cn_public.csv --dry-run
```

## 4. 正式导入

```powershell
python tools/import_public_patents.py --file D:\data\cn_public.csv --batch-size 200
```

可选参数：
- `--api http://127.0.0.1:3000/api/patents/import`
- `--format csv|json|jsonl`
- `--source public_bulk`

## 5. 验证效果

1. 打开 `/search` 页面
2. 选择 `本地数据库` 模式
3. 输入中文关键词或专利号检索

说明：在线模式在中文查询且未配置 CNIPR 时，现已优先尝试本地公开数据缓存。
