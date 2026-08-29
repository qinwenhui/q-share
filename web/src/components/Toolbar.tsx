import { Component } from 'solid-js';
import type { SortKey, SortOrder } from '../types';
import { t } from '../i18n';

interface Props {
  sort: SortKey;
  order: SortOrder;
  onSort: (s: SortKey) => void;
}

export const Toolbar: Component<Props> = (props) => {
  const arrow = () => (props.order === 'asc' ? '↑' : '↓');

  return (
    <div class="listing-header" role="row">
      <button
        type="button"
        onClick={() => props.onSort('name')}
        aria-sort={props.sort === 'name' ? (props.order === 'asc' ? 'ascending' : 'descending') : 'none'}
      >
        {t('browse.sort.name')} {props.sort === 'name' ? arrow() : ''}
      </button>
      <button
        type="button"
        onClick={() => props.onSort('size')}
        aria-sort={props.sort === 'size' ? (props.order === 'asc' ? 'ascending' : 'descending') : 'none'}
      >
        {t('browse.sort.size')} {props.sort === 'size' ? arrow() : ''}
      </button>
      <button
        type="button"
        class="col-modified"
        onClick={() => props.onSort('modified')}
        aria-sort={props.sort === 'modified' ? (props.order === 'asc' ? 'ascending' : 'descending') : 'none'}
      >
        {t('browse.sort.modified')} {props.sort === 'modified' ? arrow() : ''}
      </button>
    </div>
  );
};

export default Toolbar;