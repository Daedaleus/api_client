import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Api } from './api';
import type { AppEvent, CommandAck } from './types';

/** Name of the Tauri event, see `EVENT_NAME` in src-tauri/src/lib.rs. */
const EVENT_NAME = 'app-event';

/** The backend inside the Tauri window. */
export const tauriApi: Api = {
  execute: (command) => invoke<CommandAck>('execute', { command }),
  onEvent: (handler) => listen<AppEvent>(EVENT_NAME, (event) => handler(event.payload)),
};
