// Settings route — `/settings`
//
// Single page that exposes every theme/background/density knob the user
// can tune. Each control writes back to the theme store immediately; the
// preview on the rest of the SPA updates live.
//
// Layout: a centred column with sectioned groups (Appearance / Background
// / Data). No per-control labels using `.label` — they're stacked above
// their control so we can shrink the side gutter on narrow viewports.

import { Component, Show, createSignal, For } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import {
  currentTheme,
  setTheme,
  setBackground,
  resetTheme,
  exportTheme,
  importTheme,
  type ThemeId,
  type Density,
} from '../stores/theme';
import { whoami } from '../api';
import { lang, setLanguage, t, type Lang } from '../i18n';

const MAX_UPLOAD_BYTES = 5 * 1024 * 1024;

export const Settings: Component = () => {
  const nav = useNavigate();
  const [importStatus, setImportStatus] = createSignal<'' | 'ok' | 'fail'>('');
  const [uploadError, setUploadError] = createSignal<string>('');
  const [who, setWho] = createSignal<Awaited<ReturnType<typeof whoami>> | null>(null);

  // Fire-and-forget whoami on mount for the "About" footer.
  whoami().then(setWho).catch(() => {});

  const onUpload = (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    setUploadError('');
    if (file.size > MAX_UPLOAD_BYTES) {
      const mb = (file.size / 1024 / 1024).toFixed(1);
      setUploadError(t('settings.upload.too_large', { size: mb }));
      input.value = '';
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      const data = String(reader.result);
      setBackground({ type: 'data', dataUrl: data });
    };
    reader.onerror = () => setUploadError(t('settings.upload.read_fail'));
    reader.readAsDataURL(file);
  };

  const onExport = () => {
    const blob = new Blob([exportTheme()], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `qshare-theme-${Date.now()}.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  const onImportFile = (e: Event) => {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const ok = importTheme(String(reader.result));
      setImportStatus(ok ? 'ok' : 'fail');
      setTimeout(() => setImportStatus(''), 2000);
    };
    reader.readAsText(file);
    input.value = '';
  };

  // ── Translate theme / density labels so we don't have to maintain two
  // copies of those arrays for EN vs ZH. `t()` is called inside the
  // render rather than at array-build time so it picks up live language
  // changes (the original code froze the strings on first mount, which
  // meant switching language in the same session never updated these
  // chip labels).
  const THEMES: { id: ThemeId }[] = [
    { id: 'midnight' },
    { id: 'daylight' },
    { id: 'forest' },
    { id: 'sunset' },
    { id: 'sakura' },
  ];

  const DENSITIES: { id: Density }[] = [
    { id: 'compact' },
    { id: 'normal' },
    { id: 'cozy' },
  ];

  return (
    <div class="settings-pane">
      <header class="preview-bar">
        <button class="back" onClick={() => nav(-1)} title={t('settings.back')}>
          {t('settings.back')}
        </button>
        <span class="path">{t('settings.title')}</span>
        <button
          class="settings-reset"
          onClick={() => {
            if (confirm(t('settings.reset.confirm'))) resetTheme();
          }}
        >
          {t('settings.reset')}
        </button>
      </header>

      <div class="settings-body">
        {/* ───── Appearance ───── */}
        <section class="settings-group">
          <h2>{t('settings.appearance')}</h2>

          <div class="setting-row">
            <label class="setting-label">{t('settings.theme')}</label>
            <div class="theme-picker">
              <For each={THEMES}>
                {(th) => (
                  <button
                    type="button"
                    class={`theme-chip${currentTheme().id === th.id ? ' on' : ''}`}
                    aria-pressed={currentTheme().id === th.id}
                    title={t(`theme.${th.id}.hint`)}
                    onClick={() => setTheme({ id: th.id })}
                  >
                    <span class={`theme-swatch theme-swatch-${th.id}`} aria-hidden="true" />
                    <span class="theme-chip-label">{t(`theme.${th.id}`)}</span>
                  </button>
                )}
              </For>
            </div>
          </div>

          <div class="setting-row">
            <label class="setting-label" for="accent-input">{t('settings.accent')}</label>
            <div class="setting-control">
              <input
                id="accent-input"
                type="color"
                value={currentTheme().accent}
                onInput={(e) => setTheme({ accent: e.currentTarget.value })}
              />
              <input
                type="text"
                class="accent-hex"
                value={currentTheme().accent}
                spellcheck={false}
                onChange={(e) => {
                  const v = e.currentTarget.value.trim();
                  if (/^#[0-9a-fA-F]{6}$/.test(v)) setTheme({ accent: v });
                }}
              />
            </div>
          </div>

          <div class="setting-row">
            <label class="setting-label" for="radius-input">{t('settings.radius')}</label>
            <div class="setting-control">
              <input
                id="radius-input"
                type="range"
                min="0"
                max="20"
                value={currentTheme().radius}
                onInput={(e) => setTheme({ radius: Number(e.currentTarget.value) })}
              />
              <span class="setting-readout">{currentTheme().radius}px</span>
            </div>
          </div>

          <div class="setting-row">
            <label class="setting-label">{t('settings.density')}</label>
            <div class="setting-control">
              <div class="density-picker">
                <For each={DENSITIES}>
                  {(d) => (
                    <button
                      type="button"
                      class={`density-btn${currentTheme().density === d.id ? ' on' : ''}`}
                      aria-pressed={currentTheme().density === d.id}
                      onClick={() => setTheme({ density: d.id })}
                    >
                      {t(`density.${d.id}`)}
                    </button>
                  )}
                </For>
              </div>
            </div>
          </div>
        </section>

        {/* ───── Background ───── */}
        <section class="settings-group">
          <h2>{t('settings.background')}</h2>

          <div class="setting-row">
            <label class="setting-label">{t('settings.bg.type')}</label>
            <div class="setting-control">
              <div class="bg-type-picker">
                <button
                  type="button"
                  class={`density-btn${currentTheme().background.type === 'none' ? ' on' : ''}`}
                  onClick={() => setBackground({ type: 'none' })}
                >{t('settings.bg.none')}</button>
                <button
                  type="button"
                  class={`density-btn${currentTheme().background.type === 'url' ? ' on' : ''}`}
                  onClick={() => setBackground({ type: 'url' })}
                >{t('settings.bg.url')}</button>
                <button
                  type="button"
                  class={`density-btn${currentTheme().background.type === 'data' ? ' on' : ''}`}
                  onClick={() => setBackground({ type: 'data' })}
                >{t('settings.bg.upload')}</button>
              </div>
            </div>
          </div>

          <Show when={currentTheme().background.type === 'url'}>
            <div class="setting-row">
              <label class="setting-label" for="bg-url">{t('settings.bg.image_url')}</label>
              <div class="setting-control">
                <input
                  id="bg-url"
                  type="url"
                  placeholder="https://…"
                  value={currentTheme().background.url ?? ''}
                  onChange={(e) => setBackground({ url: e.currentTarget.value })}
                />
              </div>
            </div>
          </Show>

          <Show when={currentTheme().background.type === 'data'}>
            <div class="setting-row">
              <label class="setting-label" for="bg-upload">{t('settings.bg.upload_image')}</label>
              <div class="setting-control">
                <input
                  id="bg-upload"
                  type="file"
                  accept="image/*"
                  onChange={onUpload}
                />
                <Show when={uploadError()}>
                  <span class="setting-error">{uploadError()}</span>
                </Show>
                <Show when={currentTheme().background.dataUrl && !uploadError()}>
                  <span class="setting-readout">{t('settings.bg.uploaded')}</span>
                </Show>
              </div>
            </div>
          </Show>

          <Show when={currentTheme().background.type !== 'none'}>
            <div class="setting-row">
              <label class="setting-label" for="bg-blur">{t('settings.bg.blur')}</label>
              <div class="setting-control">
                <input
                  id="bg-blur"
                  type="range"
                  min="0"
                  max="20"
                  value={currentTheme().background.blur}
                  onInput={(e) => setBackground({ blur: Number(e.currentTarget.value) })}
                />
                <span class="setting-readout">{currentTheme().background.blur}px</span>
              </div>
            </div>

            <div class="setting-row">
              <label class="setting-label" for="bg-overlay">{t('settings.bg.overlay')}</label>
              <div class="setting-control">
                <input
                  id="bg-overlay"
                  type="range"
                  min="0"
                  max="1"
                  step="0.05"
                  value={currentTheme().background.overlay}
                  onInput={(e) => setBackground({ overlay: Number(e.currentTarget.value) })}
                />
                <span class="setting-readout">
                  {Math.round(currentTheme().background.overlay * 100)}%
                </span>
              </div>
            </div>
          </Show>
        </section>

        {/* ───── Language ───── */}
        <section class="settings-group">
          <h2>{t('settings.language')}</h2>
          <div class="setting-row">
            <label class="setting-label">{t('settings.language')}</label>
            <div class="setting-control">
              <div class="density-picker">
                <button
                  type="button"
                  class={`density-btn${lang() === 'en' ? ' on' : ''}`}
                  aria-pressed={lang() === 'en'}
                  onClick={() => setLanguage('en')}
                >{t('settings.language.en')}</button>
                <button
                  type="button"
                  class={`density-btn${lang() === 'zh-CN' ? ' on' : ''}`}
                  aria-pressed={lang() === 'zh-CN'}
                  onClick={() => setLanguage('zh-CN')}
                >{t('settings.language.zh-CN')}</button>
              </div>
            </div>
          </div>
        </section>

        {/* ───── Data ───── */}
        <section class="settings-group">
          <h2>{t('settings.data')}</h2>

          <div class="setting-row">
            <label class="setting-label">{t('settings.data.export_import')}</label>
            <div class="setting-control">
              <button type="button" onClick={onExport}>{t('settings.export')}</button>
              <label class="settings-import">
                {t('settings.import')}
                <input type="file" accept="application/json" onChange={onImportFile} />
              </label>
              <Show when={importStatus() === 'ok'}>
                <span class="setting-readout" style="color: var(--success)">{t('settings.imported')}</span>
              </Show>
              <Show when={importStatus() === 'fail'}>
                <span class="setting-error">{t('settings.import.fail')}</span>
              </Show>
            </div>
          </div>
        </section>

        {/* ───── About ───── */}
        <section class="settings-group">
          <h2>{t('settings.about')}</h2>
          <Show when={who()} fallback={<p class="setting-readout">{t('settings.about.loading')}</p>}>
            {(w) => (
              <dl class="about-list">
                <dt>{t('settings.about.server')}</dt><dd>{w().version}</dd>
                <dt>{t('settings.about.shared_root')}</dt><dd class="mono">{w().shared_root}</dd>
                <dt>{t('settings.about.url')}</dt><dd class="mono">{w().url}</dd>
                <dt>{t('settings.about.ip')}</dt><dd class="mono">{w().client_ip}</dd>
              </dl>
            )}
          </Show>
        </section>
      </div>
    </div>
  );
};

export default Settings;
