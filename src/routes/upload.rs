use super::AppState;
use axum::{extract::State, Json};
use serde_json::json;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10 MB
const MAX_PDF_STORE_SIZE: usize = 20 * 1024 * 1024; // 20 MB

/// POST /api/upload/pdf-store — 上传 PDF 文件并存储，返回可预览的 URL
pub async fn api_upload_pdf_store(
    _state: State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Json<serde_json::Value> {
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut file_name = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field.file_name().unwrap_or("unknown.pdf").to_lowercase();
            match field.bytes().await {
                Ok(data) => {
                    if data.len() > MAX_PDF_STORE_SIZE {
                        return Json(json!({"status": "error", "message": "文件大小超过 20MB 限制"}));
                    }
                    file_bytes = data.to_vec();
                }
                Err(_) => return Json(json!({"status": "error", "message": "文件读取失败"})),
            }
        }
    }

    if file_bytes.is_empty() {
        return Json(json!({"status": "error", "message": "缺少文件"}));
    }

    // 仅允许 PDF 文件
    let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    if ext != "pdf" {
        return Json(json!({"status": "error", "message": "仅支持 PDF 文件"}));
    }

    // 确保上传目录存在
    let upload_dir = "data/uploads";
    if let Err(e) = std::fs::create_dir_all(upload_dir) {
        return Json(json!({"status": "error", "message": format!("创建上传目录失败: {}", e)}));
    }

    // 用 UUID 命名文件
    let uuid = uuid::Uuid::new_v4().to_string();
    let filename = format!("{}.pdf", uuid);
    let filepath = format!("{}/{}", upload_dir, filename);

    if let Err(e) = std::fs::write(&filepath, &file_bytes) {
        return Json(json!({"status": "error", "message": format!("保存文件失败: {}", e)}));
    }

    let url = format!("/uploads/{}", filename);
    Json(json!({
        "status": "ok",
        "url": url,
        "filename": filename,
        "size": file_bytes.len(),
    }))
}

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
            file_name = field.file_name().unwrap_or("unknown.txt").to_lowercase();
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
    let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();

    let is_image = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"
    );

    let file_content = if is_image {
        // For images, use AI vision to describe the content
        let ai_client = s.config.read().unwrap_or_else(|e| e.into_inner()).ai_client();
        match describe_image_with_ai(&ai_client, &file_bytes, &ext).await {
            Ok(description) => description,
            Err(e) => return Json(json!({"error": format!("图片识别失败: {}", e)})),
        }
    } else if ext == "pdf" {
        match extract_pdf_text(&file_bytes) {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => {
                return Json(
                    json!({"error": "PDF 无法提取文字。请直接输入专利号或使用「粘贴文本」功能。"}),
                )
            }
            Err(e) => return Json(json!({"error": format!("PDF 提取失败: {}", e)})),
        }
    } else if ext == "docx" {
        // DOCX = ZIP containing XML; extract text from word/document.xml
        match extract_docx_text(&file_bytes) {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => return Json(json!({"error": "DOCX 文件无可提取的文字内容"})),
            Err(e) => return Json(json!({"error": format!("DOCX 解析失败: {}", e)})),
        }
    } else if ext == "doc" {
        return Json(
            json!({"error": "暂不支持旧版 .doc 格式，请将文件另存为 .docx、.txt 或 .pdf 后重试"}),
        );
    } else {
        // TXT, CSV, etc. — try UTF-8, then GBK
        match String::from_utf8(file_bytes.clone()) {
            Ok(text) => text,
            Err(_) => {
                // Try GBK/GB18030 for Chinese text files
                let (text, _encoding, had_errors) = encoding_rs::GBK.decode(&file_bytes);
                if had_errors {
                    return Json(
                        json!({"error": "文件编码不支持，请上传 UTF-8 或 GBK 编码的文本文件、.docx、PDF 或图片"}),
                    );
                }
                text.into_owned()
            }
        }
    };

    if file_content.trim().is_empty() {
        return Json(json!({"error": "文件内容为空"}));
    }

    let ai_client = s.config.read().unwrap_or_else(|e| e.into_inner()).ai_client();

    let file_type_label = if is_image {
        "图片识别内容"
    } else {
        "上传文档"
    };

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

/// 通用文件内容提取（首页上传附件用）
pub async fn api_upload_extract(
    State(s): State<AppState>,
    mut multipart: axum::extract::Multipart,
) -> Json<serde_json::Value> {
    let mut file_bytes: Vec<u8> = Vec::new();
    let mut file_name = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field.file_name().unwrap_or("unknown.txt").to_lowercase();
            match field.bytes().await {
                Ok(data) => {
                    if data.len() > MAX_FILE_SIZE {
                        return Json(json!({"error": "文件大小超过 10MB 限制"}));
                    }
                    file_bytes = data.to_vec();
                }
                Err(_) => return Json(json!({"error": "文件读取失败"})),
            }
        }
    }

    if file_bytes.is_empty() {
        return Json(json!({"error": "缺少文件"}));
    }

    let ext = file_name.rsplit('.').next().unwrap_or("").to_lowercase();
    let is_image = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp");

    let text = if is_image {
        let ai_client = s.config.read().unwrap_or_else(|e| e.into_inner()).ai_client();
        match describe_image_with_ai(&ai_client, &file_bytes, &ext).await {
            Ok(desc) => desc,
            Err(e) => return Json(json!({"error": format!("图片识别失败: {}", e)})),
        }
    } else if ext == "pdf" {
        match extract_pdf_text(&file_bytes) {
            Ok(t) if !t.trim().is_empty() => t,
            Ok(_) => return Json(json!({"error": "该 PDF 无法提取文字。请直接输入专利号，系统会自动在线获取专利内容；或使用「粘贴文本」功能手动输入。"})),
            Err(e) => return Json(json!({"error": format!("PDF 提取失败: {}。请直接输入专利号或使用「粘贴文本」功能。", e)})),
        }
    } else if ext == "docx" {
        match extract_docx_text(&file_bytes) {
            Ok(t) if !t.trim().is_empty() => t,
            Ok(_) => return Json(json!({"error": "DOCX 无可提取文字"})),
            Err(e) => return Json(json!({"error": format!("DOCX 解析失败: {}", e)})),
        }
    } else if ext == "doc" {
        return Json(json!({"error": "暂不支持 .doc 格式，请另存为 .docx 或 .pdf"}));
    } else {
        match String::from_utf8(file_bytes.clone()) {
            Ok(t) => t,
            Err(_) => {
                let (t, _, had_errors) = encoding_rs::GBK.decode(&file_bytes);
                if had_errors {
                    return Json(json!({"error": "文件编码不支持"}));
                }
                t.into_owned()
            }
        }
    };

    Json(json!({
        "text": text.chars().take(50000).collect::<String>(),
        "file_type": ext,
        "length": text.len()
    }))
}

/// Extract text from a PDF file: pdf-extract → PyMuPDF → Tesseract OCR
fn extract_pdf_text(data: &[u8]) -> Result<String, String> {
    // Step 1: Rust pdf-extract
    if let Ok(text) = pdf_extract::extract_text_from_mem(data) {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }
    // Step 2: PyMuPDF
    if let Ok(text) = extract_pdf_text_pymupdf(data) {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }
    // Step 3: Tesseract OCR (handles scanned/special font PDFs)
    extract_pdf_text_ocr(data)
}

/// Fallback: use Python PyMuPDF (fitz) to extract text from PDF
fn extract_pdf_text_pymupdf(data: &[u8]) -> Result<String, String> {
    use std::io::Write;

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("innoforge_pdf_{}.pdf", std::process::id()));
    let tmp_str = tmp_path.to_string_lossy().to_string();

    // Write PDF bytes to temp file
    let mut f = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("创建临时文件失败: {}", e))?;
    f.write_all(data)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    drop(f);

    let python = r"C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe";
    let script = format!(
        "import fitz,sys\nsys.stdout.reconfigure(encoding='utf-8')\ndoc=fitz.open(sys.argv[1])\nfor p in doc:\n print(p.get_text())",
    );

    let output = std::process::Command::new(python)
        .args(["-c", &script, &tmp_str])
        .output();

    // Clean up temp file
    let _ = std::fs::remove_file(&tmp_path);

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            if text.trim().is_empty() {
                Ok(String::new()) // empty — let caller try next method
            } else {
                Ok(text)
            }
        }
        Ok(_) | Err(_) => Ok(String::new()), // failed — let caller try next method
    }
}

/// Fallback: use Tesseract OCR via Python to extract text from scanned PDFs
fn extract_pdf_text_ocr(data: &[u8]) -> Result<String, String> {
    use std::io::Write;

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("innoforge_ocr_{}.pdf", std::process::id()));
    let tmp_str = tmp_path.to_string_lossy().to_string();

    let mut f = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("创建临时文件失败: {}", e))?;
    f.write_all(data)
        .map_err(|e| format!("写入临时文件失败: {}", e))?;
    drop(f);

    let python = r"C:\Users\Administrator\AppData\Local\Programs\Python\Python313\python.exe";
    let script = r#"
import pytesseract, fitz, sys
from PIL import Image
import io

sys.stdout.reconfigure(encoding='utf-8')
pytesseract.pytesseract.tesseract_cmd = r'C:\Program Files\Tesseract-OCR\tesseract.exe'
doc = fitz.open(sys.argv[1])
for page in doc:
    mat = fitz.Matrix(2.0, 2.0)
    pix = page.get_pixmap(matrix=mat)
    img = Image.open(io.BytesIO(pix.tobytes('png')))
    text = pytesseract.image_to_string(img, lang='chi_sim+eng')
    print(text)
"#;

    let output = std::process::Command::new(python)
        .args(["-c", script, &tmp_str])
        .output();

    let _ = std::fs::remove_file(&tmp_path);

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            if text.trim().is_empty() {
                Err("OCR 也无法识别文字".into())
            } else {
                Ok(text)
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            Err(format!("OCR 失败: {}", stderr.chars().take(200).collect::<String>()))
        }
        Err(e) => Err(format!("无法调用 Python OCR: {}", e)),
    }
}

/// Extract text from a DOCX file (ZIP containing XML)
fn extract_docx_text(data: &[u8]) -> Result<String, String> {
    use std::io::{Cursor, Read};
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("非有效DOCX: {}", e))?;
    let mut xml = String::new();
    if let Ok(mut file) = archive.by_name("word/document.xml") {
        file.read_to_string(&mut xml)
            .map_err(|e| format!("读取失败: {}", e))?;
    } else {
        return Err("DOCX 中找不到 word/document.xml".into());
    }
    // Strip XML tags to get plain text
    let mut text = String::new();
    let mut in_tag = false;
    for ch in xml.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(ch);
        }
    }
    Ok(text)
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
