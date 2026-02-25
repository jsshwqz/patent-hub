@echo off
chcp 65001 >nul
echo 正在导入测试专利数据...
echo.

curl -X POST http://127.0.0.1:3000/api/patents/import ^
  -H "Content-Type: application/json" ^
  -d "{\"patents\":[{\"id\":\"test-001\",\"patent_number\":\"CN114398997A\",\"title\":\"一种基于人工智能的专利检索方法\",\"abstract_text\":\"本发明公开了一种基于人工智能的专利检索方法，通过深度学习模型对专利文本进行语义分析，提高检索准确率。\",\"description\":\"详细描述...\",\"claims\":\"1. 一种基于人工智能的专利检索方法，其特征在于...\",\"applicant\":\"北京智能科技有限公司\",\"inventor\":\"张三;李四\",\"filing_date\":\"2021-12-15\",\"publication_date\":\"2022-06-28\",\"grant_date\":null,\"ipc_codes\":\"G06F16/33\",\"cpc_codes\":\"G06F16/3344\",\"priority_date\":\"2021-12-15\",\"country\":\"CN\",\"kind_code\":\"A\",\"family_id\":null,\"legal_status\":\"公开\",\"citations\":\"[]\",\"cited_by\":\"[]\",\"source\":\"test\",\"raw_json\":\"{}\",\"created_at\":\"2026-02-25T10:00:00Z\"},{\"id\":\"test-002\",\"patent_number\":\"US11234567B2\",\"title\":\"Artificial Intelligence Patent Search System\",\"abstract_text\":\"An AI-powered patent search system using natural language processing and machine learning.\",\"description\":\"Detailed description...\",\"claims\":\"1. A patent search system comprising...\",\"applicant\":\"Tech Innovations Inc.\",\"inventor\":\"John Smith\",\"filing_date\":\"2020-05-10\",\"publication_date\":\"2022-01-25\",\"grant_date\":\"2022-01-25\",\"ipc_codes\":\"G06F16/33\",\"cpc_codes\":\"G06F16/3344\",\"priority_date\":\"2020-05-10\",\"country\":\"US\",\"kind_code\":\"B2\",\"family_id\":null,\"legal_status\":\"Granted\",\"citations\":\"[]\",\"cited_by\":\"[]\",\"source\":\"test\",\"raw_json\":\"{}\",\"created_at\":\"2026-02-25T10:00:00Z\"},{\"id\":\"test-003\",\"patent_number\":\"CN113268512A\",\"title\":\"智能燃烧器控制系统\",\"abstract_text\":\"本发明涉及一种智能燃烧器控制系统，包括温度传感器、控制器和执行机构，能够自动调节燃烧效率。\",\"description\":\"详细描述...\",\"claims\":\"1. 一种智能燃烧器控制系统，其特征在于包括...\",\"applicant\":\"王马芝\",\"inventor\":\"王马芝\",\"filing_date\":\"2021-06-20\",\"publication_date\":\"2021-08-17\",\"grant_date\":null,\"ipc_codes\":\"F23N5/00\",\"cpc_codes\":\"F23N5/00\",\"priority_date\":\"2021-06-20\",\"country\":\"CN\",\"kind_code\":\"A\",\"family_id\":null,\"legal_status\":\"公开\",\"citations\":\"[]\",\"cited_by\":\"[]\",\"source\":\"test\",\"raw_json\":\"{}\",\"created_at\":\"2026-02-25T10:00:00Z\"}]}"

echo.
echo 导入完成！
echo 现在可以搜索 "人工智能" 或 "燃烧器" 测试
pause
