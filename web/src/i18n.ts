// Lightweight i18n — Chinese (Simplified) and English.
//
// The whole app reads every user-visible string through `t('key')`. The
// helper returns the string for the active language, falling back to
// English if a translation is missing.
//
// Persistence: `qshare.lang.v1` in localStorage. The default is the
// browser's `navigator.language` if it's `zh*`, otherwise English.
//
// Adding a new string: add it under `en` and (ideally) under `zh`. If
// you only add it under `en`, the English text is used everywhere —
// the missing-translation case is handled, not crashed.

import { createSignal } from 'solid-js';

export type Lang = 'en' | 'zh-CN';

const STORAGE_KEY = 'qshare.lang.v1';

function detect(): Lang {
  if (typeof navigator === 'undefined') return 'en';
  const n = navigator.language.toLowerCase();
  if (n.startsWith('zh')) return 'zh-CN';
  return 'en';
}

function load(): Lang {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === 'en' || v === 'zh-CN') return v;
  } catch {}
  return detect();
}

export const [lang, setLang] = createSignal<Lang>(load());

export function setLanguage(l: Lang) {
  setLang(l);
  try {
    localStorage.setItem(STORAGE_KEY, l);
  } catch {}
  // Notify the rest of the app. We mutate the document language so the
  // browser's own UI (e.g. native confirm dialogs) inherits the choice.
  if (typeof document !== 'undefined') {
    document.documentElement.lang = l === 'zh-CN' ? 'zh-CN' : 'en';
  }
}

// ─────────────────────────────────────────────────────────────────────
// Dictionaries
// ─────────────────────────────────────────────────────────────────────

const en = {
  // Sidebar
  'sidebar.search.placeholder': 'search files…',
  'sidebar.search.recursive': 'recursive',
  'sidebar.search.scope.here': 'here',
  'sidebar.search.scope.root': 'whole share',
  'sidebar.favorites.title': 'favorites',
  'sidebar.favorites.empty': 'pin a path to see it here',
  'sidebar.favorites.add': 'pin',
  'sidebar.favorites.added': 'pinned',
  'sidebar.favorites.remove': 'unpin',
  'sidebar.favorites.pin_title': 'pin to sidebar',
  'sidebar.favorites.unpin_title': 'unpin from sidebar',
  'sidebar.favorites.nothing': 'nothing to pin',
  'sidebar.settings': 'settings',

  // Theme preset names + hints (Settings → appearance)
  'theme.midnight': 'Midnight',
  'theme.midnight.hint': 'deep blue-black',
  'theme.daylight': 'Daylight',
  'theme.daylight.hint': 'crisp whites, clear blue accent',
  'theme.forest': 'Forest',
  'theme.forest.hint': 'green-leaning, calm',
  'theme.sunset': 'Sunset',
  'theme.sunset.hint': 'warm oranges on purple-black',
  'theme.sakura': 'Sakura',
  'theme.sakura.hint': 'soft pink, anime-friendly',

  // Density labels (Settings → appearance)
  'density.compact': 'Compact',
  'density.normal': 'Normal',
  'density.cozy': 'Cozy',

  // Browse
  'browse.up': 'parent directory',
  'browse.loading': 'loading…',
  'browse.empty': 'empty directory',
  'browse.sort.name': 'name',
  'browse.sort.size': 'size',
  'browse.sort.modified': 'modified',
  'browse.toolbar.sort': 'sort',

  // Context menu
  'menu.open': 'open',
  'menu.download': 'download',
  'menu.details': 'details…',

  // Details modal
  'details.title': 'file details',
  'details.path': 'path',
  'details.size': 'size',
  'details.modified': 'modified',
  'details.permissions': 'permissions',
  'details.type': 'type',
  'details.md5': 'md5',
  'details.copy': 'copy',
  'details.copied': '✓',
  'details.compute': 'compute',
  'details.hashing': 'hashing…',
  'details.hash.md5': 'MD5',
  'details.hash.sha256': 'SHA-256',
  'details.md5.placeholder': '— (file too large; compute below)',
  'details.close': 'close',
  'details.directory': 'directory',

  // Settings
  'settings.title': 'settings',
  'settings.back': '← back',
  'settings.reset': 'reset',
  'settings.appearance': 'appearance',
  'settings.theme': 'theme',
  'settings.accent': 'accent',
  'settings.radius': 'corner radius',
  'settings.density': 'density',
  'settings.background': 'background',
  'settings.bg.type': 'type',
  'settings.bg.none': 'none',
  'settings.bg.url': 'url',
  'settings.bg.upload': 'upload',
  'settings.bg.image_url': 'image url',
  'settings.bg.upload_image': 'upload image',
  'settings.bg.uploaded': 'uploaded',
  'settings.bg.blur': 'blur',
  'settings.bg.overlay': 'overlay',
  'settings.data': 'data',
  'settings.data.export_import': 'export / import',
  'settings.export': 'export theme…',
  'settings.import': 'import…',
  'settings.imported': 'imported ✓',
  'settings.import.fail': 'invalid file',
  'settings.about': 'about',
  'settings.about.server': 'server',
  'settings.about.shared_root': 'shared root',
  'settings.about.url': 'your URL',
  'settings.about.ip': 'your IP',
  'settings.about.loading': 'loading…',
  'settings.reset.confirm': 'Reset theme to default?',
  'settings.upload.too_large': 'file too large ({size} MB; max 5 MB)',
  'settings.upload.read_fail': 'failed to read file',

  // Language section
  'settings.language': 'language',
  'settings.language.en': 'English',
  'settings.language.zh-CN': '简体中文',

  // Misc
  'action.cancel': 'cancel',
  'action.ok': 'ok',
} as const;

const zh: Record<keyof typeof en, string> = {
  // Sidebar
  'sidebar.search.placeholder': '搜索文件…',
  'sidebar.search.recursive': '递归',
  'sidebar.search.scope.here': '当前目录',
  'sidebar.search.scope.root': '整个共享',
  'sidebar.favorites.title': '收藏夹',
  'sidebar.favorites.empty': '将目录固定到这里以便快速访问',
  'sidebar.favorites.add': '固定',
  'sidebar.favorites.added': '已固定',
  'sidebar.favorites.remove': '取消固定',
  'sidebar.favorites.pin_title': '固定到侧栏',
  'sidebar.favorites.unpin_title': '从侧栏移除',
  'sidebar.favorites.nothing': '当前目录无法固定',
  'sidebar.settings': '设置',

  // 主题预设名称 + 描述(设置 → 外观)
  'theme.midnight': '午夜',
  'theme.midnight.hint': '深蓝黑色调',
  'theme.daylight': '白昼',
  'theme.daylight.hint': '清爽白底,蓝色强调',
  'theme.forest': '森林',
  'theme.forest.hint': '绿色基调,宁静',
  'theme.sunset': '日落',
  'theme.sunset.hint': '紫黑底暖橙色',
  'theme.sakura': '樱花粉',
  'theme.sakura.hint': '柔和粉色,二次元友好',

  // 密度(设置 → 外观)
  'density.compact': '紧凑',
  'density.normal': '适中',
  'density.cozy': '宽松',

  // Browse
  'browse.up': '上一级',
  'browse.loading': '加载中…',
  'browse.empty': '空目录',
  'browse.sort.name': '名称',
  'browse.sort.size': '大小',
  'browse.sort.modified': '修改时间',
  'browse.toolbar.sort': '排序',

  // Context menu
  'menu.open': '打开',
  'menu.download': '下载',
  'menu.details': '详情…',

  // Details modal
  'details.title': '文件详情',
  'details.path': '路径',
  'details.size': '大小',
  'details.modified': '修改时间',
  'details.permissions': '权限',
  'details.type': '类型',
  'details.md5': 'MD5',
  'details.copy': '复制',
  'details.copied': '✓',
  'details.compute': '计算',
  'details.hashing': '计算中…',
  'details.hash.md5': 'MD5',
  'details.hash.sha256': 'SHA-256',
  'details.md5.placeholder': '— (文件过大,请在下方计算)',
  'details.close': '关闭',
  'details.directory': '目录',

  // Settings
  'settings.title': '设置',
  'settings.back': '← 返回',
  'settings.reset': '重置',
  'settings.appearance': '外观',
  'settings.theme': '主题',
  'settings.accent': '强调色',
  'settings.radius': '圆角',
  'settings.density': '密度',
  'settings.background': '背景',
  'settings.bg.type': '类型',
  'settings.bg.none': '无',
  'settings.bg.url': '网址',
  'settings.bg.upload': '上传',
  'settings.bg.image_url': '图片网址',
  'settings.bg.upload_image': '上传图片',
  'settings.bg.uploaded': '已上传',
  'settings.bg.blur': '模糊',
  'settings.bg.overlay': '蒙版',
  'settings.data': '数据',
  'settings.data.export_import': '导出 / 导入',
  'settings.export': '导出主题…',
  'settings.import': '导入…',
  'settings.imported': '已导入 ✓',
  'settings.import.fail': '无效文件',
  'settings.about': '关于',
  'settings.about.server': '服务端',
  'settings.about.shared_root': '共享根',
  'settings.about.url': '访问地址',
  'settings.about.ip': '你的 IP',
  'settings.about.loading': '加载中…',
  'settings.reset.confirm': '将主题恢复为默认值?',
  'settings.upload.too_large': '文件过大 ({size} MB;最大 5 MB)',
  'settings.upload.read_fail': '读取文件失败',

  // Language section
  'settings.language': '语言',
  'settings.language.en': 'English',
  'settings.language.zh-CN': '简体中文',

  // Misc
  'action.cancel': '取消',
  'action.ok': '确定',
};

export type Key = keyof typeof en;
const dicts: Record<Lang, Record<string, string>> = { en, 'zh-CN': zh };

/** Translate a key. Falls back to English if the active language doesn't
 *  have a translation, then to the key itself if even English is missing.
 *  `key` is typed as `string` (not the strict `Key` union) so callers can
 *  pass template-literal keys like `` `theme.${th.id}` `` when iterating
 *  over a runtime-known list of ids — the dictionary lookup is the same
 *  either way. */
export function t(key: string, vars?: Record<string, string | number>): string {
  const d = dicts[lang()]?.[key] ?? dicts.en[key] ?? key;
  if (!vars) return d;
  return d.replace(/\{(\w+)\}/g, (_, k) => String(vars[k] ?? `{${k}}`));
}
