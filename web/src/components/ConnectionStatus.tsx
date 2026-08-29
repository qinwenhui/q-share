import { Component } from 'solid-js';
import { wsStatus } from '../stores/live';

export const ConnectionStatus: Component = () => {
  const color = () =>
    wsStatus() === 'live'
      ? 'var(--success)'
      : wsStatus() === 'connecting'
      ? 'var(--fg-faint)'
      : 'var(--danger)';
  const label = () =>
    wsStatus() === 'live' ? 'live' : wsStatus() === 'connecting' ? 'connecting' : 'offline';

  return (
    <span
      title={`realtime updates: ${label()}`}
      style={{
        display: 'inline-flex',
        'align-items': 'center',
        gap: '6px',
        'font-size': '12px',
        color: 'var(--fg-dim)',
      }}
    >
      <span
        style={{
          width: '8px',
          height: '8px',
          'border-radius': '50%',
          background: color(),
          'box-shadow': wsStatus() === 'live' ? `0 0 6px ${color()}` : 'none',
        }}
      />
      {label()}
    </span>
  );
};

export default ConnectionStatus;