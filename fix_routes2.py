#!/usr/bin/env python3
# -*- coding: utf-8 -*-

# 读取文件
with open('src/routes.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

# 找到损坏函数的开始和结束
start_idx = -1
end_idx = -1

for i, line in enumerate(lines):
    if '// 推荐相似专利' in line and start_idx == -1:
        start_idx = i
    if start_idx >= 0 and '// 上传文件对比' in line:
        end_idx = i
        break

print(f"Found function from line {start_idx} to {end_idx}")

if start_idx >= 0 and end_idx >= 0:
    # 新的正确函数
    new_function_lines = [
        '// 推荐相似专利\n',
        'pub async fn api_recommend_similar(\n',
        '    Path(id): Path<String>,\n',
        '    State(s): State<AppState>,\n',
        ') -> Json<serde_json::Value> {\n',
        '    let patent = match s.db.get_patent(&id) {\n',
        '        Ok(Some(p)) => p,\n',
        '        _ => return Json(json!({"error": "专利不存在"})),\n',
        '    };\n',
        '    \n',
        '    // 使用标题关键词搜索相似专利\n',
        '    let keywords: Vec<&str> = patent.title.split_whitespace().take(5).collect();\n',
        '    let query = keywords.join(" ");\n',
        '    \n',
        '    let req = SearchRequest { \n',
        '        query, \n',
        '        page: 1, \n',
        '        page_size: 10, \n',
        '        country: None, \n',
        '        date_from: None, \n',
        '        date_to: None,\n',
        '        search_type: None,\n',
        '    };\n',
        '    \n',
        '    match api_search_online(State(s), Json(req)).await {\n',
        '        Json(result) => {\n',
        '            if let Some(patents) = result.get("patents").and_then(|p| p.as_array()) {\n',
        '                let filtered: Vec<_> = patents.iter()\n',
        '                    .filter(|p| p.get("id").and_then(|i| i.as_str()) != Some(&id))\n',
        '                    .take(5)\n',
        '                    .collect();\n',
        '                Json(json!({"similar": filtered}))\n',
        '            } else {\n',
        '                Json(json!({"similar": []}))\n',
        '            }\n',
        '        }\n',
        '    }\n',
        '}\n',
        '\n',
    ]
    
    # 替换损坏的部分
    new_lines = lines[:start_idx] + new_function_lines + lines[end_idx:]
    
    # 写回文件
    with open('src/routes.rs', 'w', encoding='utf-8') as f:
        f.writelines(new_lines)
    
    print(f"Successfully replaced lines {start_idx} to {end_idx}")
    print(f"Old: {end_idx - start_idx} lines, New: {len(new_function_lines)} lines")
else:
    print("Could not find function boundaries!")
