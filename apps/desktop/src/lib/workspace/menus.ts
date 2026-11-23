// The entries of the sidebar's menus. Built when a menu opens, so labels
// are in the current language.

import { i18n } from '../i18n/i18n.svelte';
import { entry, separator, type MenuItem } from '../menu';
import type { CollectionView, TreeItemView } from '../types';
import type { Workspace } from './workspace.svelte';

/** "New": requests, collections and imports (opening is a button of its own). */
export function newMenu(workspace: Workspace): MenuItem[] {
  return [
    entry(
      'new-request',
      i18n.t('collection.new_request'),
      () => workspace.tabs.openScratch(),
      i18n.t('shortcut.new_request'),
    ),
    entry('new-collection', i18n.t('menu.new_collection'), () => workspace.createCollection()),
    separator,
    entry('import', i18n.t('menu.import'), [
      entry('import-postman', i18n.t('menu.import_postman'), () =>
        workspace.importCollection('postman'),
      ),
      entry('import-insomnia', i18n.t('menu.import_insomnia'), () =>
        workspace.importCollection('insomnia'),
      ),
      entry('import-openapi', i18n.t('menu.import_openapi'), () =>
        workspace.importCollection('openapi'),
      ),
      entry('import-har', i18n.t('menu.import_har'), () => workspace.importCollection('har')),
      entry('import-bruno', i18n.t('menu.import_bruno'), () => workspace.importCollection('bruno')),
      entry('import-url', i18n.t('menu.import_url'), () => workspace.importFromUrl()),
      separator,
      entry('import-curl', i18n.t('menu.import_curl'), () => workspace.curl.openImport()),
    ]),
  ];
}

/** What can be done with an open collection. */
export function collectionMenu(workspace: Workspace, collection: CollectionView): MenuItem[] {
  const id = collection.id;
  const unsaved: MenuItem[] = collection.temporary
    ? [
        entry('save-as', i18n.t('menu.save_collection_as'), () => workspace.saveCollectionAs(id)),
        separator,
      ]
    : [];
  return [
    ...unsaved,
    entry('new-request', i18n.t('collection.new_request'), () => workspace.newRequest(id, id)),
    entry('new-folder', i18n.t('collection.new_folder'), () => workspace.newFolder(id, id)),
    entry('settings', i18n.t('collection.variables'), () => workspace.defaults.open(id, id)),
    entry('export-postman', i18n.t('collection.export_postman'), () =>
      workspace.exportCollection(id),
    ),
    separator,
    entry('close', i18n.t('collection.close'), () => workspace.closeCollection(id)),
  ];
}

/** What can be done with a folder or request in the tree. */
export function treeItemMenu(
  workspace: Workspace,
  collectionId: string,
  item: TreeItemView,
): MenuItem[] {
  const common: MenuItem[] = [
    entry('rename', i18n.t('collection.rename'), () => workspace.rename(collectionId, item)),
    entry('duplicate', i18n.t('collection.duplicate'), () =>
      workspace.duplicate(collectionId, item),
    ),
    entry('move', i18n.t('collection.move'), [
      entry('move-up', i18n.t('collection.move_up'), () =>
        workspace.moveBy(collectionId, item, -1),
      ),
      entry('move-down', i18n.t('collection.move_down'), () =>
        workspace.moveBy(collectionId, item, 1),
      ),
      entry('move-to', i18n.t('collection.move_to'), () => workspace.moveTo(collectionId, item)),
    ]),
    separator,
    entry('delete', i18n.t('collection.delete'), () => workspace.remove(collectionId, item)),
  ];
  if (item.kind !== 'folder') return common;
  return [
    entry('new-request', i18n.t('collection.new_request'), () =>
      workspace.newRequest(collectionId, item.id),
    ),
    entry('new-folder', i18n.t('collection.new_folder'), () =>
      workspace.newFolder(collectionId, item.id),
    ),
    entry('settings', i18n.t('collection.folder_variables'), () =>
      workspace.defaults.open(collectionId, item.id),
    ),
    separator,
    ...common,
  ];
}
