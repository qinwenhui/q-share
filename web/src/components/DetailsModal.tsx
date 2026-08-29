// Details modal — shown from the row/tile context menu's "Details…"
// action. Calls /api/stat for the entry; if `md5` is null (file > 64 MB)
// the user can still trigger an on-demand hash via /api/hash and we
// stream the result back in.
//
// Designed for both files and directories — directories just don't get
// the MD5 / SHA-256 / hash-button row.

import { Component, Show, createMemo, createResource, createSignal } from 'solid-js';
import { Portal } from 'solid-js/web';
import { hashFile, statFile } from '../api';
import { formatBytes } from '../utils/format';
import { FileIcon } from './FileIcon';
import { t } from '../i18n';

interface Props {
  path: string;
  visible: boolean;
  onClose: () => void;
}

type HashAlgo = 'md5' | 'sha256';

export const DetailsModal: Component<Props> = (props) => {
  // Always call /api/stat — pass `path` so Solid refetches when the host
  // opens the modal for a different entry.
  const [stat] = createResource(
    () => (props.visible ? props.path : null),
    (path) => statFile(path),
  );

  const [hashAlgo, setHashAlgo] = createSignal<HashAlgo>('md5');
  const [hashing, setHashing] = createSignal(false);
  const [hashResult, setHashResult] = createSignal<string | null>(null);
  const [hashError, setHashError] = createSignal<string | null>(null);
  const [copied, setCopied] = createSignal<'path' | 'md5' | null>(null);

  const resetTransient = () => {
    setHashResult(null);
    setHashError(null);
    setHashing(false);
    setCopied(null);
  };
  createMemo(() => {
    if (props.visible) resetTransient();
  });

  const onHash = async () => {
    const s = stat();
    if (!s) return;
    setHashing(true);
    setHashError(null);
    setHashResult(null);
    try {
      const hex = await hashFile(s.path, hashAlgo());
      setHashResult(hex);
    } catch (e) {
      setHashError(e instanceof Error ? e.message : String(e));
    } finally {
      setHashing(false);
    }
  };

  const onCopy = async (value: string, kind: 'path' | 'md5') => {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(kind);
      window.setTimeout(() => setCopied((c) => (c === kind ? null : c)), 1200);
    } catch {
      // clipboard rejected (insecure context, etc.) — silently no-op;
      // users can still select the text by hand.
    }
  };

  return (
    <Show when={props.visible}>
      <Portal>
        <div
          class="modal-backdrop"
          role="dialog"
          aria-modal="true"
          aria-label="file details"
          onClick={(e) => {
            if (e.target === e.currentTarget) props.onClose();
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') props.onClose();
          }}
        >
          <div class="modal details-modal">
            <Show
              when={!stat.loading && stat()}
              fallback={
                <Show when={stat.error} fallback={<div class="modal-loading">{t('settings.about.loading')}</div>}>
                  <div class="modal-loading error">{(stat.error as Error).message}</div>
                </Show>
              }
            >
              {(s) => {
                const modified = () => new Date(s().modified * 1000);
                return (
                  <>
                    <header class="details-head">
                      <div class="details-icon">
                        <FileIcon isDir={s().is_dir} mime={s().mime} name={s().name} size="lg" />
                      </div>
                      <div class="details-titles">
                        <div class="details-name" title={s().name}>{s().name}</div>
                        <div class="details-sub">
                          {s().is_dir ? t('details.directory') : (s().mime ?? 'file')}
                        </div>
                      </div>
                      <button
                        type="button"
                        class="modal-x"
                        aria-label={t('details.close')}
                        onClick={props.onClose}
                      >×</button>
                    </header>

                    <dl class="details-grid">
                      <dt>{t('details.path')}</dt>
                      <dd class="mono">
                        <span class="truncate" title={s().path}>{s().path}</span>
                        <button class="copy-btn" onClick={() => onCopy(s().path, 'path')}>
                          {copied() === 'path' ? t('details.copied') : t('details.copy')}
                        </button>
                      </dd>

                      <dt>{t('details.size')}</dt>
                      <dd class="mono">
                        {s().is_dir ? '—' : `${formatBytes(s().size)} (${s().size.toLocaleString()} B)`}
                      </dd>

                      <dt>{t('details.modified')}</dt>
                      <dd class="mono">
                        {modified().toLocaleString()} <span class="muted">({modified().toLocaleDateString()})</span>
                      </dd>

                      <dt>{t('details.permissions')}</dt>
                      <dd class="mono">{s().mode || '—'}</dd>

                      <Show when={!s().is_dir && s().mime}>
                        <dt>{t('details.type')}</dt>
                        <dd class="mono">{s().mime}</dd>
                      </Show>

                      <Show when={!s().is_dir}>
                        <dt>{t('details.md5')}</dt>
                        <dd class="mono">
                          <Show when={s().md5} fallback={<span class="muted">{t('details.md5.placeholder')}</span>}>
                            <span class="hash">{s().md5}</span>
                            <button class="copy-btn" onClick={() => onCopy(s().md5 ?? '', 'md5')}>
                              {copied() === 'md5' ? t('details.copied') : t('details.copy')}
                            </button>
                          </Show>
                        </dd>
                      </Show>
                    </dl>

                    <Show when={!s().is_dir}>
                      <div class="details-hash">
                        <div class="hash-controls">
                          <label>
                            <input
                              type="radio"
                              name="hash-algo"
                              checked={hashAlgo() === 'md5'}
                              onChange={() => setHashAlgo('md5')}
                            />
                            {t('details.hash.md5')}
                          </label>
                          <label>
                            <input
                              type="radio"
                              name="hash-algo"
                              checked={hashAlgo() === 'sha256'}
                              onChange={() => setHashAlgo('sha256')}
                            />
                            {t('details.hash.sha256')}
                          </label>
                          <button
                            type="button"
                            class="primary"
                            disabled={hashing()}
                            onClick={onHash}
                          >
                            {hashing() ? t('details.hashing') : t('details.compute')}
                          </button>
                        </div>
                        <Show when={hashResult()}>
                          <div class="hash-result mono">{hashResult()}</div>
                        </Show>
                        <Show when={hashError()}>
                          <div class="hash-result mono error">{hashError()}</div>
                        </Show>
                      </div>
                    </Show>

                    <footer class="details-foot">
                      <button
                        type="button"
                        class="primary"
                        onClick={() => {
                          const a = document.createElement('a');
                          a.href = `/api/raw?path=${encodeURIComponent(s().path)}`;
                          a.download = s().name;
                          a.click();
                        }}
                      >{t('menu.download')}</button>
                      <button type="button" onClick={props.onClose}>{t('details.close')}</button>
                    </footer>
                  </>
                );
              }}
            </Show>
          </div>
        </div>
      </Portal>
    </Show>
  );
};

export default DetailsModal;
