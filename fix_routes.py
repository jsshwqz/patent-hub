#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import re

# 读取文件
with open('src/routes.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 新的正确函数
new_function = '''// 推荐相似专利
pub async fn api_recommend_similar(
    Path(id): Path<String>,
    State(s): State<AppState>,
) -> Json<serde_json::Value> {
    let patent = match s.db.get_patent(&id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"error": "专利不存在"})),
    };
    
    // 使用标题关键词搜索相似专利
    let keywords: Vec<&str> = patent.title.split_whitespace().take(5).collect();
    let query = keywords.join(" ");
    
    let req = SearchRequest { 
        query, 
        page: 1, 
        page_size: 10, 
        country: None, 
        date_from: None, 
        date_to: None,
        search_type: None,
    };
    
    match api_search_online(State(s), Json(req)).await {
        Json(result) => {
            if let Some(patents) = result.get("patents").and_then(|p| p.as_array()) {
                let filtered: Vec<_> = patents.iter()
                    .filter(|p| p.get("id").and_then(|i| i.as_str()) != Some(&id))
                    .take(5)
                    .collect();
                Json(json!({"similar": filtered}))
            } else {
                Json(json!({"similar": []}))
            }
        }
    }
}

'''

# 使用正则表达式替换损坏的函数
# 匹配从 "// 推荐相似专利" 到下一个 "// 上传文件对比" 之前的所有内容
pattern = r'// 推荐相似专利.*?(?=// 上传文件对比)'
content = re.sub(pattern, new_function, content, flags=re.DOTALL)

# 写回文件
with open('src/routes.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed routes.rs successfully!")
