import { Component, Show, Switch, Match, createResource, onMount, createSignal } from 'solid-js';
import { useParams, useNavigate } from '@solidjs/router';
import { downloadUrl, statFile } from '../api';
import { previewKindOf, type PreviewKind } from '../types';
import { formatBytes } from '../utils/format';

export const Preview: Component = () => {
  const params = useParams<{ path: string }>();
  const navigate = useNavigate();
  const path = () => decodeURIComponent(params.path);

  const [stat] = createResource(path, async (p) => {
    try {
      return await statFile(p);
    } catch (e) {
      return null;
    }
  });

  const kind = (): PreviewKind => previewKindOf(stat()?.mime);

  const onKey = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      navigate(-1);
    }
  };

  onMount(() => {
    document.addEventListener('keydown', onKey);
  });

  return (
    <div class="preview-pane">
      <header class="preview-bar">
        <button class="back" onClick={() => navigate(-1)} title="Back (Esc)">
          ← back
        </button>
        <span class="path" title={path()}>
          {path()}
        </span>
        <Show when={stat()}>
          {(s) => (
            <>
              <span class="meta">{formatBytes(s().size)}</span>
              <a class="dl" href={downloadUrl(s().path)} download={s().name}>
                download
              </a>
            </>
          )}
        </Show>
      </header>

      <main class="preview-body">
        <Show
          when={stat()}
          fallback={<div class="loading">loading…</div>}
        >
          {(s) => (
            <Show
              when={!s().is_dir}
              fallback={<div class="empty">directories can't be previewed</div>}
            >
              <Switch>
                <Match when={kind() === 'image'}>
                  <img class="preview-image" src={downloadUrl(s().path)} alt={s().name} />
                </Match>
                <Match when={kind() === 'video'}>
                  <video class="preview-video" controls preload="metadata" src={downloadUrl(s().path)} />
                </Match>
                <Match when={kind() === 'audio'}>
                  <div class="preview-audio-wrap">
                    <div class="preview-audio-icon">♪</div>
                    <audio controls preload="metadata" src={downloadUrl(s().path)} />
                  </div>
                </Match>
                <Match when={kind() === 'pdf'}>
                  <iframe
                    class="preview-pdf"
                    src={downloadUrl(s().path)}
                    title={s().name}
                  />
                </Match>
                <Match when={kind() === 'text'}>
                  <TextPreview path={s().path} />
                </Match>
                <Match when={kind() === 'other'}>
                  <div class="empty">
                    no inline preview available
                    <br />
                    <a href={downloadUrl(s().path)} download={s().name}>
                      download {s().name}
                    </a>
                  </div>
                </Match>
              </Switch>
            </Show>
          )}
        </Show>
      </main>
    </div>
  );
};

const TextPreview: Component<{ path: string }> = (props) => {
  const [text] = createResource(
    () => props.path,
    async (p) => {
      const r = await fetch(downloadUrl(p));
      if (!r.ok) throw new Error(`${r.status}`);
      const blob = await r.blob();
      // Cap at 2MB to avoid huge files hanging the UI.
      if (blob.size > 2 * 1024 * 1024) {
        return { truncated: true, body: await blob.text().then((t) => t.slice(0, 2 * 1024 * 1024)) };
      }
      return { truncated: false, body: await blob.text() };
    },
  );

  return (
    <Show when={text()} fallback={<div class="loading">loading…</div>}>
      {(t) => (
        <pre class="preview-text">
          <code>{t().body}</code>
          <Show when={t().truncated}>
            <div class="preview-truncated">— truncated at 2 MB; download for full file —</div>
          </Show>
        </pre>
      )}
    </Show>
  );
};

export default Preview;
