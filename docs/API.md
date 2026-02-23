# API Documentation / API 文档

## Base URL / 基础 URL

```
http://127.0.0.1:3000
```

## Endpoints / 接口

### 1. Search Patents / 搜索专利

**POST** `/api/search`

Search patents online or in local database.

#### Request Body

```json
{
  "query": "artificial intelligence",
  "mode": "online",
  "country": "US",
  "date_from": "2020-01-01",
  "date_to": "2024-12-31"
}
```

Parameters:
- `query` (string, required): Search keywords / 搜索关键词
- `mode` (string, required): "online" or "local" / "在线" 或 "本地"
- `country` (string, optional): Country code (US, CN, EP, etc.) / 国家代码
- `date_from` (string, optional): Start date (YYYY-MM-DD) / 起始日期
- `date_to` (string, optional): End date (YYYY-MM-DD) / 结束日期

#### Response

```json
{
  "patents": [
    {
      "id": "uuid",
      "patent_id": "US1234567B2",
      "title": "Method and system for...",
      "abstract": "This invention relates to...",
      "applicant": "Company Name",
      "inventor": "John Doe",
      "filing_date": "2020-01-15",
      "publication_date": "2022-03-20",
      "country": "US",
      "url": "https://patents.google.com/patent/US1234567B2"
    }
  ],
  "total": 100
}
```

### 2. Get Patent Details / 获取专利详情

**GET** `/api/patent/:id`

Get detailed information about a specific patent.

#### Parameters

- `id` (path): Patent UUID / 专利 UUID

#### Response

```json
{
  "id": "uuid",
  "patent_id": "US1234567B2",
  "title": "Method and system for...",
  "abstract": "This invention relates to...",
  "applicant": "Company Name",
  "inventor": "John Doe",
  "filing_date": "2020-01-15",
  "publication_date": "2022-03-20",
  "country": "US",
  "url": "https://patents.google.com/patent/US1234567B2",
  "claims": "1. A method comprising...",
  "description": "Detailed description..."
}
```

### 3. AI Analysis / AI 分析

**POST** `/api/ai/analyze`

Analyze a patent using AI.

#### Request Body

```json
{
  "patent_id": "uuid",
  "analysis_type": "summary"
}
```

Parameters:
- `patent_id` (string, required): Patent UUID / 专利 UUID
- `analysis_type` (string, optional): "summary", "technical", "claims" / 分析类型

#### Response

```json
{
  "analysis": "This patent describes a novel approach to...",
  "key_points": [
    "Main innovation: ...",
    "Technical advantage: ...",
    "Potential applications: ..."
  ]
}
```

### 4. Compare Patents / 对比专利

**POST** `/api/ai/compare`

Compare two patents using AI.

#### Request Body

```json
{
  "patent_id_1": "uuid1",
  "patent_id_2": "uuid2"
}
```

Parameters:
- `patent_id_1` (string, required): First patent UUID / 第一个专利 UUID
- `patent_id_2` (string, required): Second patent UUID / 第二个专利 UUID

#### Response

```json
{
  "comparison": {
    "similarities": [
      "Both patents address...",
      "Similar technical approach..."
    ],
    "differences": [
      "Patent 1 focuses on...",
      "Patent 2 uses a different method..."
    ],
    "conclusion": "Overall assessment..."
  }
}
```

### 5. Similar Patents / 相似专利

**GET** `/api/patent/:id/similar`

Get similar patents based on keywords.

#### Parameters

- `id` (path): Patent UUID / 专利 UUID
- `limit` (query, optional): Number of results (default: 5) / 结果数量

#### Response

```json
{
  "similar_patents": [
    {
      "id": "uuid",
      "patent_id": "US7654321B2",
      "title": "Related invention...",
      "similarity_score": 0.85
    }
  ]
}
```

### 6. Search History / 搜索历史

**GET** `/api/search/history`

Get recent search history.

#### Parameters

- `limit` (query, optional): Number of records (default: 10) / 记录数量

#### Response

```json
{
  "history": [
    {
      "id": 1,
      "query": "artificial intelligence",
      "timestamp": "2024-12-24T10:30:00Z",
      "result_count": 100
    }
  ]
}
```

### 7. Export Data / 导出数据

**POST** `/api/export`

Export search results to CSV.

#### Request Body

```json
{
  "patent_ids": ["uuid1", "uuid2", "uuid3"]
}
```

Parameters:
- `patent_ids` (array, required): List of patent UUIDs / 专利 UUID 列表

#### Response

Returns CSV file with headers:
```
Patent ID,Title,Applicant,Inventor,Filing Date,Publication Date,Country,URL
```

### 8. Statistics / 统计

**GET** `/api/stats`

Get statistics for current search results.

#### Parameters

- `query` (query, optional): Filter by search query / 按搜索查询过滤

#### Response

```json
{
  "total_patents": 1000,
  "top_applicants": [
    {"name": "Company A", "count": 150},
    {"name": "Company B", "count": 120}
  ],
  "country_distribution": [
    {"country": "US", "count": 500},
    {"country": "CN", "count": 300}
  ],
  "yearly_trend": [
    {"year": 2020, "count": 100},
    {"year": 2021, "count": 150}
  ]
}
```

## Error Responses / 错误响应

All endpoints may return error responses:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE"
}
```

Common error codes:
- `INVALID_REQUEST`: Invalid request parameters / 无效请求参数
- `NOT_FOUND`: Resource not found / 资源未找到
- `API_ERROR`: External API error / 外部 API 错误
- `DATABASE_ERROR`: Database operation failed / 数据库操作失败
- `AI_ERROR`: AI service error / AI 服务错误

## Rate Limiting / 速率限制

Currently no rate limiting is implemented. For production use, consider:
- Implementing rate limiting per IP
- Using API keys for authentication
- Setting up request quotas

## Authentication / 认证

Currently no authentication is required. For production use, consider:
- API key authentication
- OAuth 2.0
- JWT tokens

## CORS / 跨域

CORS is enabled for all origins in development. For production:
- Restrict allowed origins
- Configure allowed methods
- Set appropriate headers

## WebSocket Support / WebSocket 支持

Not currently implemented. Future consideration for:
- Real-time search updates
- Live AI analysis streaming
- Collaborative features
