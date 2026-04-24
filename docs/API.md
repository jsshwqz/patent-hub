# API 文档 / API Documentation

## 基础 URL / Base URL

```
http://127.0.0.1:3000
```

> 默认端口为 `3000`，可通过环境变量 `INNOFORGE_PORT` 覆盖（例如 `http://127.0.0.1:3900`）。

## 说明 / Notes

- 搜索历史为前端 `localStorage`，没有 `/api/search/history` 接口。
- 详情页是页面路由 `GET /patent/:id`，不是 JSON API。
- AI 分析相关接口均为 OpenAI 兼容下游能力封装。

---

## 页面路由 / Page Routes

| Method | Path | Description |
|---|---|---|
| GET | `/` | 首页 |
| GET | `/search` | 搜索页 |
| GET | `/compare` | 专利对比页 |
| GET | `/ai` | AI 助手页 |
| GET | `/settings` | 设置页 |
| GET | `/patent/:id` | 专利详情页 |
| GET | `/test` | 调试测试页 |
| GET | `/import` | 样例数据导入页 |

---

## 搜索接口 / Search APIs

### 1) 本地搜索 / Local Search

**POST** `/api/search`

Request:

```json
{
  "query": "人工智能",
  "page": 1,
  "page_size": 20,
  "country": "CN",
  "date_from": "2020-01-01",
  "date_to": "2024-12-31",
  "search_type": "inventor",
  "sort_by": "relevance"
}
```

Fields:
- `query` (required)
- `page` (optional, default `1`)
- `page_size` (optional, default `20`)
- `country` (optional)
- `date_from`, `date_to` (optional, `YYYY-MM-DD`)
- `search_type` (optional): `applicant | inventor | patent_number | keyword`
- `sort_by` (optional): `relevance | new | old`

Response:

```json
{
  "patents": [
    {
      "id": "uuid",
      "patent_number": "CN1234567A",
      "title": "示例标题",
      "abstract_text": "摘要...",
      "applicant": "示例申请人",
      "inventor": "示例发明人",
      "filing_date": "2024-01-01",
      "country": "CN",
      "relevance_score": 92.5,
      "score_source": "发明人包含匹配"
    }
  ],
  "total": 123,
  "page": 1,
  "page_size": 20,
  "search_type": "inventor"
}
```

### 2) 在线搜索 / Online Search

**POST** `/api/search/online`

Request 与 `/api/search` 相同。优先走 SerpAPI，失败自动回落本地搜索。

Response（SerpAPI 命中）:

```json
{
  "patents": [],
  "total": 0,
  "page": 1,
  "page_size": 10,
  "source": "serpapi"
}
```

Response（本地回落）:

```json
{
  "patents": [],
  "total": 0,
  "page": 1,
  "page_size": 20,
  "source": "local"
}
```

### 3) 统计 / Stats

**POST** `/api/search/stats`

Request 与 `/api/search` 相同（用于保持筛选一致）。

Response:

```json
{
  "total": 100,
  "applicants": [["公司A", 20], ["公司B", 15]],
  "countries": [["CN", 50], ["US", 30]],
  "years": [["2020", 10], ["2021", 20]]
}
```

### 4) 导出 CSV / Export CSV

**POST** `/api/search/export`

Request 与 `/api/search` 相同，返回 `text/csv` 文件流。

### 5) AI 分析搜索结果 / AI Analyze Search Results

**POST** `/api/search/analyze`

Request:

```json
{
  "query": "机器视觉",
  "patents": [
    { "title": "A", "abstract_text": "..." },
    { "title": "B", "abstract_text": "..." }
  ]
}
```

Response:

```json
{
  "status": "ok",
  "analysis": {}
}
```

---

## 专利接口 / Patent APIs

### 1) 按专利号抓取 / Fetch Patent by Number

**POST** `/api/patent/fetch`

Request:

```json
{
  "patent_number": "EP1234567",
  "source": "epo"
}
```

`source`: `epo | uspto`（默认 `epo`）

### 2) 批量导入 / Import Patents

**POST** `/api/patents/import`

Request:

```json
{
  "patents": []
}
```

说明：
- 推荐配合 `tools/import_public_patents.py` 使用，可将公开公告数据包（CSV/JSON/JSONL）批量导入本地库。
- 无 CNIPR 授权时，可通过该方式构建本地主检索链路。

### 3) 丰富专利信息 / Enrich Patent

**GET** `/api/patent/enrich/:id`

### 4) 相似专利推荐 / Similar Patents

**GET** `/api/patent/similar/:id`

### 5) 上传文档对比 / Upload Compare

**POST** `/api/upload/compare` (`multipart/form-data`)

Fields:
- `file` (`.txt` 等文本文件)
- `patent_id`

---

## AI 接口 / AI APIs

### 1) 对话 / Chat

**POST** `/api/ai/chat`

```json
{
  "message": "请分析该专利创新点",
  "patent_id": "uuid-optional"
}
```

### 2) 摘要 / Summarize

**POST** `/api/ai/summarize`

```json
{
  "patent_number": "CN1234567A"
}
```

### 3) 对比 / Compare

**POST** `/api/ai/compare`

```json
{
  "patent_id1": "uuid-or-number-1",
  "patent_id2": "uuid-or-number-2"
}
```

---

## 配置接口 / Settings APIs

### 1) 读取配置 / Get Settings

**GET** `/api/settings`

返回脱敏密钥与配置状态:

```json
{
  "serpapi_key": "abcd****wxyz",
  "serpapi_key_configured": true,
  "ai_base_url": "https://open.bigmodel.cn/api/paas/v4",
  "ai_api_key": "abcd****wxyz",
  "ai_api_key_configured": true,
  "ai_model": "glm-4-flash"
}
```

### 2) 保存 SerpAPI Key / Save SerpAPI

**POST** `/api/settings/serpapi`

```json
{ "api_key": "your-serpapi-key" }
```

### 3) 保存 AI 配置 / Save AI Config

**POST** `/api/settings/ai`

```json
{
  "base_url": "https://open.bigmodel.cn/api/paas/v4",
  "api_key": "your-ai-key",
  "model": "glm-4-flash"
}
```

---

## 通用错误 / Common Errors

```json
{
  "status": "error",
  "message": "..."
}
```
