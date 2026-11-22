import { open, save } from '@tauri-apps/plugin-dialog';

import type { Api } from './api';
import { commands, events } from './bindings';

/** The backend inside the Tauri window, through the generated bindings. */
export const tauriApi: Api = {
  execute: (command) => commands.execute(command),
  query: (query) => commands.query(query),
  assist: (request) => commands.assist(request),
  importUrl: (url) => commands.importUrl(url),
  onEvent: (handler) => events.appEventMessage.listen((event) => handler(event.payload)),
  pickFolder: async (options = {}) => {
    const { title } = options;
    const picked = await open({
      directory: true,
      multiple: false,
      ...(title === undefined ? {} : { title }),
    });
    return typeof picked === 'string' ? picked : null;
  },
  pickFile: async (options = {}) => {
    const picked = await open({ directory: false, multiple: false, ...options });
    return typeof picked === 'string' ? picked : null;
  },
  pickSaveFile: async (options = {}) => {
    const picked = await save(options);
    return typeof picked === 'string' ? picked : null;
  },
};
