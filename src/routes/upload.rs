use super::AppState;
use axum::{extract::State, Json};
use serde_json::json;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB

pub async fn api_upload_compare(
    State(s): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Json<serde_json::Value> {
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut file_name = String::new();
    let mut patent_id = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            file_name = field
                .file_name()
                .unwrap_or("unknown.txt")
                .to_lowercase();
            match field.bytes().await {
                Ok(data) => {
                    if data.len() > MAX_FILE_SIZE {
                        return Json(json!({"error": "文件大小超过 10MB 限制"}));
                    }
                    file_bytes = data.to_vec();
                }
                Err(_) => return Json(json!({"error": "文件读取失败"})),
            }
        } else if name == "patent_id" {
            if let Ok(text) = field.text().await {
                patent_id = text;
            }
        }
    }

    if file_bytes.is_empty() || patent_id.is_empty() {
        return Json(json!({"error": "缺少文件或专利 ID"}));
    }

    let patent = match s.db.get_patent(&patent_id) {
        Ok(Some(p)) => p,
        _ => return Json(json!({"error": "专利不存在"})),
    };

    // Extract text content based on file type
    let ext = file_name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();

    let is_image = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp");

    let file_content = if is_image {
        // For images, use AI vision to describe the content
        let ai_client = s.config.read().unwrap().ai_client();
        match describe_image_with_ai(&ai_client, &file_bytes, &ext).await {
            Ok(description) => description,
            Err(e) => return Json(json!({"error": format!("图片识别失败: {}", e)})),
        }
    } else if ext == "pdf" {
        match extract_pdf_text(&file_bytes) {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => return Json(json!({"error": "PDF 文件无可提取的文字内容（可能是扫描件），请转换为文本后重试"})),
            Err(e) => return Json(json!({"error": format!("PDF 解析失败: {}", e)})),
        }
    } else {
        // TXT, DOC, etc. — treat as UTF-8 text
        match String::from_utf8(file_bytes) {
            Ok(text) => text,
            Err(_) => return Json(json!({"error": "文件编码不支持，请上传 UTF-8 文本文件、PDF 或图片"})),
        }
    };

    if file_content.trim().is_empty() {
        return Json(json!({"error": "文件内容为空"}));
    }

    let ai_client = s.config.read().unwrap().ai_client();

    let file_type_label = if is_image { "图片识别内容" } else { "上传文档" };

    let prompt = format!(
        "请对比以下两份技术文档，分析它们的相似性和差异：\n\n\
        【专利文档】\n标题：{}\n摘要：{}\n权利要求：{}\n\n\
        【{}】\n{}\n\n\
        请从以下方面分析：\n\
        1. 技术领域是否相同\n\
        2. 解决的技术问题是否相似\n\
        3. 技术方案的相似度（百分比）\n\
        4. 是否存在侵权风险\n\
        5. 主要差异点",
        patent.title,
        patent.abstract_text,
        patent.claims.chars().take(2000).collect::<String>(),
        file_type_label,
        file_content.chars().take(3000).collect::<String>()
    );

    match ai_client.chat(&prompt, None).await {
        Ok(response) => Json(json!({
            "success": true,
            "analysis": response,
            "file_type": ext,
            "content_length": file_content.len()
        })),
        Err(e) => Json(json!({"error": format!("AI 分析失败: {}", e)})),
    }
}

/// Extract text from a PDF file using pdf-extract
fn extract_pdf_text(data: &[u8]) -> Result<String, String> {
    pdf_extract::extract_text_from_mem(data).map_err(|e| format!("{}", e))
}

/// Use AI vision (GLM-4V or compatible) to describe an image
async fn describe_image_with_ai(
    ai_client: &crate::ai::AiClient,
    image_bytes: &[u8],
    ext: &str,
) -> Result<String, String> {
    use base64::Engine;

    let b64 = base64::engine::general_purpose::STANDARD.encode(image_bytes);
    let mime = match ext {
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        _ => "image/jpeg",
    };
    let data_url = format!("data:{};base64,{}", mime, b64);

    ai_client
        .describe_image(&data_url)
        .await
        .map_err(|e| format!("{}", e))
}
