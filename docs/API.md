# API Documentation / API 文档

## Base URL / 基础 URL

```
http://127.0.0.1:3000
```

## Notes / 说明

- 搜索历史为前端 `localStorage`，没有 `/api/search/history` 接口。
- 详情页是页面路由 `GET /patent/:id`，不是 JSON API。
- AI 分析相关接口均为 OpenAI 兼容下游能力封装。

---

## Page Routes / 页面路由

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

## Search APIs / 搜索接口

### 1) Local Search / 本地搜索

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

### 2) Online Search / 在线搜索

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

### 3) Stats / 统计

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

### 4) Export CSV / 导出 CSV

**POST** `/api/search/export`

Request 与 `/api/search` 相同，返回 `text/csv` 文件流。

### 5) AI Analyze Search Results / AI 分析搜索结果

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

## Patent APIs / 专利接口

### 1) Fetch Patent by Number / 按专利号抓取

**POST** `/api/patent/fetch`

Request:

```json
{
  "patent_number": "EP1234567",
  "source": "epo"
}
```

`source`: `epo | uspto`（默认 `epo`）

### 2) Import Patents / 批量导入

**POST** `/api/patents/import`

Request:

```json
{
  "patents": []
}
```

### 3) Enrich Patent / 丰富专利信息

**GET** `/api/patent/enrich/:id`

### 4) Similar Patents / 相似专利推荐

**GET** `/api/patent/similar/:id`

### 5) Upload Compare / 上传文档对比

**POST** `/api/upload/compare` (`multipart/form-data`)

Fields:
- `file` (`.txt` 等文本文件)
- `patent_id`

---

## AI APIs / AI 接口

### 1) Chat / 对话

**POST** `/api/ai/chat`

```json
{
  "message": "请分析该专利创新点",
  "patent_id": "uuid-optional"
}
```

### 2) Summarize / 摘要

**POST** `/api/ai/summarize`

```json
{
  "patent_number": "CN1234567A"
}
```

### 3) Compare / 对比

**POST** `/api/ai/compare`

```json
{
  "patent_id1": "uuid-or-number-1",
  "patent_id2": "uuid-or-number-2"
}
```

---

## Settings APIs / 配置接口

### 1) Get Settings / 读取配置

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

### 2) Save SerpAPI / 保存 SerpAPI Key

**POST** `/api/settings/serpapi`

```json
{ "api_key": "your-serpapi-key" }
```

### 3) Save AI Config / 保存 AI 配置

**POST** `/api/settings/ai`

```json
{
  "base_url": "https://open.bigmodel.cn/api/paas/v4",
  "api_key": "your-ai-key",
  "model": "glm-4-flash"
}
```

---

## Common Errors / 通用错误

```json
{
  "status": "error",
  "message": "..."
}
```

