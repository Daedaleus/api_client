import type { Api } from './api';
import { commands, events } from './bindings';

/** The backend inside the Tauri window, through the generated bindings. */
export const tauriApi: Api = {
  execute: (command) => commands.execute(command),
  query: (query) => commands.query(query),
  onEvent: (handler) => events.appEventMessage.listen((event) => handler(event.payload)),
};
