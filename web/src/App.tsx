import type { Component, JSX } from 'solid-js';
import { Router, Route, Navigate } from '@solidjs/router';
import Browse from './routes/Browse';
import Preview from './routes/Preview';
import Search from './routes/Search';
import Settings from './routes/Settings';
import { AppShell } from './components/AppShell';
import { startLive, stopLive } from './stores/live';
import { onCleanup } from 'solid-js';

const Shell: Component<{ children?: JSX.Element }> = (props) => {
  startLive();
  onCleanup(() => stopLive());
  return <AppShell>{props.children}</AppShell>;
};

const App: Component = () => {
  return (
    <Router root={Shell}>
      <Route path="/" component={() => <Navigate href="/browse/" />} />
      <Route path="/browse/*path" component={Browse} />
      <Route path="/preview/*path" component={Preview} />
      <Route path="/search" component={Search} />
      <Route path="/settings" component={Settings} />
    </Router>
  );
};

export default App;