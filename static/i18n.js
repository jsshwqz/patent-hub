// InnoForge shared i18n system
const I18N_COMMON = {
  zh: {
    'nav.home': '创研台',
    'nav.search': '技术调研',
    'nav.idea': '创新推演',
    'nav.compare': '方案对比',
    'nav.ai': 'AI 助手',
    'nav.settings': '设置',
    // AI page
    'ai.title': 'AI 助手',
    'ai.hint': '可以问我技术相关的问题：可行性分析、方案比较、专利解读、技术路线等。',
    'ai.placeholder': '输入你的问题...',
    'ai.send': '发送',
    'ai.thinking': '思考中...',
    'ai.fail': '请求失败',
    'ai.webSearch': '联网搜索',
    'ai.searching': '正在联网搜索...',
    // Compare page
    'compare.title': '方案对比',
    'compare.patent1': '专利1（ID 或专利号）',
    'compare.patent2': '专利2（ID 或专利号）',
    'compare.placeholder1': '输入专利 ID 或专利号',
    'compare.placeholder2': '输入专利 ID 或专利号',
    'compare.btn': '开始对比分析',
    'compare.analyzing': 'AI 正在对比分析中，请稍候...',
    'compare.result': '对比分析结果',
    'compare.fail': '分析失败',
    'compare.alert': '请输入两个专利的 ID 或专利号',
    // Idea page
    'idea.title': '创新推演',
    'idea.hint': '输入你的技术想法，AI 会多角度分析并检索相关专利文献，生成可行性报告。',
    'idea.titleLabel': '想法标题',
    'idea.titlePlaceholder': '用一句话概括你的想法...',
    'idea.descLabel': '详细描述',
    'idea.descPlaceholder': '详细描述你的想法，包括技术方案、应用场景、解决的问题等...',
    'idea.submit': '提交并分析',
    'idea.clear': '清空',
    'idea.analyzing': '分析中...',
    'idea.done': '分析完成',
    'idea.timeout': '分析超时（超过 3 分钟）。请检查 AI 服务是否正常运行，或在设置页面更换 AI 服务。',
    'idea.step1': '1. 搜索网络相关技术信息',
    'idea.step2': '2. 检索全球技术文献库',
    'idea.step3': '3. 搜索本地已收录文献',
    'idea.step4': '4. AI 深度分析（可能需要 60-90 秒）',
    'idea.submitting': '正在提交...',
    'idea.webResults': '网络调研结果',
    'idea.patentResults': '相关技术文献',
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
    'idea.discussTitle': '💬 继续讨论',
    'idea.generateSummary': '📋 生成总结',
    'idea.chatPlaceholder': '继续深入讨论这个方案...',
    'idea.send': '发送',
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
    // Search page
    'search.title': '技术调研',
    'search.placeholder': '输入关键词、专利号、发明人或申请人',
    'mode.local': '本地检索',
    'mode.online': '在线搜索',
    'region.auto': '自动',
    'region.cn': '中国',
    'region.intl': '国际',
    'type.auto': '智能识别',
    'type.inventor': '发明人',
    'type.applicant': '申请人',
    'type.patentNumber': '专利号',
    'type.keyword': '关键词',
    'country.all': '全部国家',
    'sort.relevance': '相关度',
    'sort.new': '最新优先',
    'sort.old': '最早优先',
    'btn.search': '搜索',
    'btn.stats': '统计分析',
    'btn.export': '导出',
    'history.title': '搜索历史',
    'history.clear': '清空',
    'stats.title': '统计分析',
    'alert.searchFail': '搜索失败',
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
    'nav.home': 'InnoForge',
    'nav.search': 'Research',
    'nav.idea': 'Deep Reasoning',
    'nav.compare': 'Compare',
    'nav.ai': 'AI Assistant',
    'nav.settings': 'Settings',
    'ai.title': 'AI Assistant',
    'ai.hint': 'Ask me about technical topics: feasibility analysis, solution comparison, patent review, technology roadmaps, etc.',
    'ai.placeholder': 'Enter your question...',
    'ai.send': 'Send',
    'ai.thinking': 'Thinking...',
    'ai.fail': 'Request failed',
    'ai.webSearch': 'Web Search',
    'ai.searching': 'Searching the web...',
    'compare.title': 'Solution Comparison',
    'compare.patent1': 'Patent 1 (ID or patent number)',
    'compare.patent2': 'Patent 2 (ID or patent number)',
    'compare.placeholder1': 'Enter patent ID or number',
    'compare.placeholder2': 'Enter patent ID or number',
    'compare.btn': 'Start Comparison',
    'compare.analyzing': 'AI is analyzing, please wait...',
    'compare.result': 'Comparison Results',
    'compare.fail': 'Analysis failed',
    'compare.alert': 'Please enter two patent IDs or numbers',
    'idea.title': 'Innovation Reasoning',
    'idea.hint': 'Enter your technical idea. AI will analyze it from multiple angles, search related patents and literature, and generate a feasibility report.',
    'idea.titleLabel': 'Idea Title',
    'idea.titlePlaceholder': 'Summarize your idea in one sentence...',
    'idea.descLabel': 'Detailed Description',
    'idea.descPlaceholder': 'Describe your idea, technical approach, use cases...',
    'idea.submit': 'Submit & Analyze',
    'idea.clear': 'Clear',
    'idea.analyzing': 'Analyzing...',
    'idea.done': 'Analysis Complete',
    'idea.timeout': 'Analysis timed out (>3 min). Check AI service or switch provider in Settings.',
    'idea.step1': '1. Searching web for related technologies',
    'idea.step2': '2. Searching global technical literature',
    'idea.step3': '3. Searching local knowledge base',
    'idea.step4': '4. AI deep analysis (60-90 seconds)',
    'idea.submitting': 'Submitting...',
    'idea.webResults': 'Web Research Results',
    'idea.patentResults': 'Related Technical Literature',
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
    'idea.discussTitle': '💬 Continue Discussion',
    'idea.generateSummary': '📋 Generate Summary',
    'idea.chatPlaceholder': 'Continue discussing this solution...',
    'idea.send': 'Send',
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
    // Search page
    'search.title': 'Research',
    'search.placeholder': 'Enter keywords, patent number, inventor or applicant',
    'mode.local': 'Local',
    'mode.online': 'Online',
    'region.auto': 'Auto',
    'region.cn': 'China',
    'region.intl': 'International',
    'type.auto': 'Smart',
    'type.inventor': 'Inventor',
    'type.applicant': 'Applicant',
    'type.patentNumber': 'Patent No.',
    'type.keyword': 'Keyword',
    'country.all': 'All Countries',
    'sort.relevance': 'Relevance',
    'sort.new': 'Newest First',
    'sort.old': 'Oldest First',
    'btn.search': 'Search',
    'btn.stats': 'Statistics',
    'btn.export': 'Export',
    'history.title': 'Search History',
    'history.clear': 'Clear',
    'stats.title': 'Statistics',
    'alert.searchFail': 'Search failed',
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

const I18N_LANG_KEY = 'innoforge_ui_lang';
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
  // Notify page-specific hooks
  if (window._onI18nLangChange) {
    window._onI18nLangChange.forEach(function(fn) { fn(); });
  }
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
  var sw = document.getElementById('lang-switch');
  if (sw) sw.value = i18nLang;
}

// activePage is stored globally for renderSidebar to use
var _activePage = '';
function renderNavbar(activePage) {
  _activePage = activePage;
  var nav = document.getElementById('global-nav');
  if (nav) nav.style.display = 'none';
}

// Render right sidebar: navigation + language switch + page-specific controls
function renderSidebar(extraHtml) {
  var el = document.getElementById('page-sidebar');
  if (!el) return;
  var fromPath = location.pathname;
  var links = [
    { href: '/', key: 'nav.home', id: 'home' },
    { href: '/idea', key: 'nav.idea', id: 'idea' },
    { href: '/search', key: 'nav.search', id: 'search' },
    { href: '/compare', key: 'nav.compare', id: 'compare' },
    { href: '/ai', key: 'nav.ai', id: 'ai' },
    { href: '/settings?from=' + encodeURIComponent(fromPath), key: 'nav.settings', id: 'settings' }
  ];
  // Navigation section
  var html = '<div class="sidebar-section">';
  html += '<div class="sidebar-nav">';
  for (var i = 0; i < links.length; i++) {
    var cls = (links[i].id === _activePage) ? ' class="active"' : '';
    html += '<a href="' + links[i].href + '"' + cls + ' data-i18n="' + links[i].key + '">' + t(links[i].key) + '</a>';
  }
  html += '</div></div>';
  // Language section
  html += '<div class="sidebar-section">';
  html += '<div class="sidebar-label">' + (i18nLang === 'zh' ? '语言' : 'Language') + '</div>';
  html += '<select onchange="setI18nLang(this.value)" id="lang-switch">';
  html += '<option value="zh"' + (i18nLang === 'zh' ? ' selected' : '') + '>中文</option>';
  html += '<option value="en"' + (i18nLang === 'en' ? ' selected' : '') + '>EN</option>';
  html += '</select>';
  html += '</div>';
  if (extraHtml) html += extraHtml;
  el.innerHTML = html;
}
