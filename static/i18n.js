// Patent Hub shared i18n system
const I18N_COMMON = {
  zh: {
    'nav.search': '专利检索',
    'nav.idea': '创意验证',
    'nav.compare': '专利对比',
    'nav.ai': 'AI 助手',
    'nav.settings': '设置',
    // AI page
    'ai.title': '专利 AI 助手',
    'ai.hint': '可以问我任何专利相关的问题：技术分析、权利要求解读、专利检索策略等。',
    'ai.placeholder': '输入你的问题...',
    'ai.send': '发送',
    'ai.thinking': '思考中...',
    'ai.fail': '请求失败',
    // Compare page
    'compare.title': '专利对比分析',
    'compare.patent1': '专利1 ID 或专利号',
    'compare.patent2': '专利2 ID 或专利号',
    'compare.placeholder1': '输入专利 ID 或专利号',
    'compare.placeholder2': '输入专利 ID 或专利号',
    'compare.btn': '开始对比分析',
    'compare.analyzing': 'AI 正在对比分析中，请稍候...',
    'compare.result': '对比分析结果',
    'compare.fail': '分析失败',
    'compare.alert': '请输入两个专利的 ID 或专利号',
    // Idea page
    'idea.title': '创意验证与增强',
    'idea.hint': '输入你的想法，系统将自动搜索网络和专利库进行对比，由 AI 分析新颖性并提供优化建议。',
    'idea.titleLabel': '想法标题',
    'idea.titlePlaceholder': '用一句话概括你的想法...',
    'idea.descLabel': '详细描述',
    'idea.descPlaceholder': '详细描述你的想法，包括技术方案、应用场景、解决的问题等...',
    'idea.submit': '提交并分析',
    'idea.clear': '清空',
    'idea.analyzing': '分析中...',
    'idea.done': '分析完成',
    'idea.timeout': '分析超时（超过 3 分钟）。请检查 AI 服务是否正常运行，或在设置页面更换 AI 服务。',
    'idea.step1': '1. 搜索网络相关信息',
    'idea.step2': '2. 搜索 Google Patents 专利库',
    'idea.step3': '3. 搜索本地专利数据库',
    'idea.step4': '4. AI 深度分析新颖性（可能需要 30-60 秒）',
    'idea.submitting': '正在提交想法...',
    'idea.webResults': '网络搜索结果',
    'idea.patentResults': '相关专利',
    'idea.history': '历史记录',
    'idea.historyEmpty': '提交想法后这里会显示历史记录',
    'idea.scoreHigh': '高度原创',
    'idea.scoreMid': '有一定新颖性',
    'idea.scoreLow': '已有较多类似方案',
    'idea.alertTitle': '请输入想法标题',
    'idea.alertDesc': '请输入详细描述',
    'idea.serverError': '服务器错误',
    'idea.submitFail': '提交失败',
    'idea.analyzeFail': '分析失败',
    'idea.analyzeError': '分析服务错误',
    // Patent detail
    'detail.analyze': 'AI 智能分析',
    'detail.analyzing': 'AI 正在分析...',
    'detail.result': 'AI 分析结果',
    'detail.fail': '分析失败',
    'detail.tabAbstract': '摘要',
    'detail.tabClaims': '权利要求',
    'detail.tabDesc': '说明书',
    'detail.tabAiChat': 'AI 问答',
    'detail.chatPlaceholder': '问我关于这个专利的任何问题...',
    'detail.send': '发送',
    'detail.upload': '上传文档对比',
    'detail.uploadHint': '上传文件与本专利进行 AI 对比分析（支持 TXT、PDF、图片）',
    'detail.uploadBtn': '开始对比',
    'detail.similar': '相似专利推荐',
    'detail.similarLoading': '加载中...',
    'detail.similarNone': '暂无相似专利',
    'detail.similarFail': '加载失败',
    'detail.enriching': '正在从 Google Patents 获取完整专利信息...',
    'detail.enrichDone': '已获取完整专利信息（权利要求、说明书等）',
    'detail.enrichFail': '获取详情失败',
    'detail.selectFile': '请选择文件',
    'detail.uploadAnalyzing': '分析中...',
    // Settings
    'settings.title': '系统设置',
    // Common
    'common.info.patent': '专利号',
    'common.info.applicant': '申请人',
    'common.info.inventor': '发明人',
    'common.info.filingDate': '申请日',
    'common.info.pubDate': '公开日',
    'common.info.grantDate': '授权日',
    'common.info.country': '国家/地区',
    'common.info.legalStatus': '法律状态',
    'common.info.basicInfo': '基本信息',
    'common.info.classification': '分类号'
  },
  en: {
    'nav.search': 'Search',
    'nav.idea': 'Idea Validation',
    'nav.compare': 'Compare',
    'nav.ai': 'AI Assistant',
    'nav.settings': 'Settings',
    'ai.title': 'Patent AI Assistant',
    'ai.hint': 'Ask me anything about patents: technical analysis, claims interpretation, search strategies, etc.',
    'ai.placeholder': 'Enter your question...',
    'ai.send': 'Send',
    'ai.thinking': 'Thinking...',
    'ai.fail': 'Request failed',
    'compare.title': 'Patent Comparison',
    'compare.patent1': 'Patent 1 ID or Number',
    'compare.patent2': 'Patent 2 ID or Number',
    'compare.placeholder1': 'Enter patent ID or number',
    'compare.placeholder2': 'Enter patent ID or number',
    'compare.btn': 'Start Comparison',
    'compare.analyzing': 'AI is analyzing, please wait...',
    'compare.result': 'Comparison Results',
    'compare.fail': 'Analysis failed',
    'compare.alert': 'Please enter two patent IDs or numbers',
    'idea.title': 'Idea Validation & Enhancement',
    'idea.hint': 'Enter your idea. The system will search the web and patent databases, then use AI to analyze novelty and provide optimization suggestions.',
    'idea.titleLabel': 'Idea Title',
    'idea.titlePlaceholder': 'Summarize your idea in one sentence...',
    'idea.descLabel': 'Detailed Description',
    'idea.descPlaceholder': 'Describe your idea, technical approach, use cases...',
    'idea.submit': 'Submit & Analyze',
    'idea.clear': 'Clear',
    'idea.analyzing': 'Analyzing...',
    'idea.done': 'Analysis Complete',
    'idea.timeout': 'Analysis timed out (>3 min). Check AI service or switch provider in Settings.',
    'idea.step1': '1. Searching web for related info',
    'idea.step2': '2. Searching Google Patents',
    'idea.step3': '3. Searching local patent database',
    'idea.step4': '4. AI deep novelty analysis (30-60 seconds)',
    'idea.submitting': 'Submitting idea...',
    'idea.webResults': 'Web Search Results',
    'idea.patentResults': 'Related Patents',
    'idea.history': 'History',
    'idea.historyEmpty': 'History will appear after submitting ideas',
    'idea.scoreHigh': 'Highly Original',
    'idea.scoreMid': 'Moderately Novel',
    'idea.scoreLow': 'Many Similar Solutions Exist',
    'idea.alertTitle': 'Please enter an idea title',
    'idea.alertDesc': 'Please enter a description',
    'idea.serverError': 'Server error',
    'idea.submitFail': 'Submission failed',
    'idea.analyzeFail': 'Analysis failed',
    'idea.analyzeError': 'Analysis service error',
    'detail.analyze': 'AI Analysis',
    'detail.analyzing': 'AI is analyzing...',
    'detail.result': 'AI Analysis Result',
    'detail.fail': 'Analysis failed',
    'detail.tabAbstract': 'Abstract',
    'detail.tabClaims': 'Claims',
    'detail.tabDesc': 'Description',
    'detail.tabAiChat': 'AI Chat',
    'detail.chatPlaceholder': 'Ask me anything about this patent...',
    'detail.send': 'Send',
    'detail.upload': 'Upload Document for Comparison',
    'detail.uploadHint': 'Upload a file to compare with this patent via AI (TXT, PDF, images supported)',
    'detail.uploadBtn': 'Start Comparison',
    'detail.similar': 'Similar Patents',
    'detail.similarLoading': 'Loading...',
    'detail.similarNone': 'No similar patents found',
    'detail.similarFail': 'Failed to load',
    'detail.enriching': 'Fetching full patent details from Google Patents...',
    'detail.enrichDone': 'Full patent details loaded (claims, description, etc.)',
    'detail.enrichFail': 'Failed to fetch details',
    'detail.selectFile': 'Please select a file',
    'detail.uploadAnalyzing': 'Analyzing...',
    'settings.title': 'System Settings',
    'common.info.patent': 'Patent No.',
    'common.info.applicant': 'Applicant',
    'common.info.inventor': 'Inventor',
    'common.info.filingDate': 'Filing Date',
    'common.info.pubDate': 'Publication Date',
    'common.info.grantDate': 'Grant Date',
    'common.info.country': 'Country/Region',
    'common.info.legalStatus': 'Legal Status',
    'common.info.basicInfo': 'Basic Information',
    'common.info.classification': 'Classification'
  }
};

const I18N_LANG_KEY = 'patent_hub_ui_lang';
let i18nLang = localStorage.getItem(I18N_LANG_KEY) || 'zh';

function t(key, vars) {
  const dict = I18N_COMMON[i18nLang] || I18N_COMMON.zh;
  let value = dict[key] || key;
  if (vars) {
    Object.keys(vars).forEach(function(k) {
      value = value.replace(new RegExp('\\{' + k + '\\}', 'g'), String(vars[k]));
    });
  }
  return value;
}

function setI18nLang(lang) {
  i18nLang = (lang === 'en') ? 'en' : 'zh';
  localStorage.setItem(I18N_LANG_KEY, i18nLang);
  applyI18nCommon();
}

function applyI18nCommon() {
  document.documentElement.lang = i18nLang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach(function(el) {
    el.textContent = t(el.getAttribute('data-i18n'));
  });
  document.querySelectorAll('[data-i18n-placeholder]').forEach(function(el) {
    el.placeholder = t(el.getAttribute('data-i18n-placeholder'));
  });
  document.querySelectorAll('[data-i18n-title]').forEach(function(el) {
    el.title = t(el.getAttribute('data-i18n-title'));
  });
}
