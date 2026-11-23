// The app's own settings (network: proxy, extra CA certificates, client
// certificates) in a dialog. A client certificate's passphrase reaches the
// dialog only encrypted; a new one is sent as `newPassphrase` and encrypted
// by the backend before it is saved. The backend applies them before saving, so settings that cannot
// work are refused with the reason. The interface language lives here too,
// but only in the web view (see i18n).

import type { Api } from '../api';
import { toUserText } from '../errors';
import { i18n } from '../i18n/i18n.svelte';
import type { SettingsView, UserText } from '../types';

export class SettingsState {
  /** The settings being edited; `null` while the dialog is closed. */
  editing = $state<SettingsView | null>(null);
  error = $state<UserText | null>(null);
  saving = $state(false);

  readonly #api: Api;

  constructor(api: Api) {
    this.#api = api;
  }

  /** Why the saved settings were not applied at start, if so (`null` otherwise or if unknown). */
  async startupProblem(): Promise<UserText | null> {
    try {
      const result = await this.#api.query({ type: 'settings' });
      return result.type === 'settings' ? result.problem : null;
    } catch (error) {
      return toUserText(error);
    }
  }

  async open(): Promise<void> {
    this.error = null;
    try {
      const result = await this.#api.query({ type: 'settings' });
      if (result.type !== 'settings') throw new Error(`unexpected answer ${result.type}`);
      this.editing = result.settings;
      this.error = result.problem;
    } catch (error) {
      // The dialog opens with the defaults, saying why.
      this.editing = { proxy: { mode: 'system' }, caCertificates: [], clientCertificates: [] };
      this.error = toUserText(error);
    }
  }

  close(): void {
    this.editing = null;
    this.error = null;
  }

  /** Lets the user pick a certificate file and adds it. */
  async addCertificate(): Promise<void> {
    const file = await this.#api.pickFile({
      title: i18n.t('settings.ca_pick'),
      filters: [
        { name: i18n.t('settings.ca_files'), extensions: ['pem', 'crt', 'cer', 'der'] },
        { name: i18n.t('import.filter_all'), extensions: ['*'] },
      ],
    });
    if (file && this.editing && !this.editing.caCertificates.includes(file)) {
      this.editing.caCertificates.push(file);
    }
  }

  removeCertificate(index: number): void {
    this.editing?.caCertificates.splice(index, 1);
  }

  /** Adds an empty client certificate entry to fill in. */
  addClientCertificate(): void {
    this.editing?.clientCertificates.push({
      host: '',
      certificate: '',
      key: '',
      encryptedPassphrase: null,
      newPassphrase: null,
    });
  }

  removeClientCertificate(index: number): void {
    this.editing?.clientCertificates.splice(index, 1);
  }

  /** Lets the user pick the certificate or key file of entry `index`. */
  async pickClientFile(index: number, which: 'certificate' | 'key'): Promise<void> {
    const file = await this.#api.pickFile({
      title: i18n.t(which === 'certificate' ? 'settings.client_pick' : 'settings.client_pick_key'),
      filters: [
        which === 'certificate'
          ? { name: i18n.t('settings.client_files'), extensions: ['pem', 'crt', 'p12', 'pfx'] }
          : { name: i18n.t('settings.client_key_files'), extensions: ['pem', 'key'] },
        { name: i18n.t('import.filter_all'), extensions: ['*'] },
      ],
    });
    const entry = this.editing?.clientCertificates[index];
    if (file && entry) entry[which] = file;
  }

  /** A typed passphrase replaces the stored one; empty input keeps it. */
  typePassphrase(index: number, text: string): void {
    const entry = this.editing?.clientCertificates[index];
    if (entry) entry.newPassphrase = text === '' ? null : text;
  }

  /** Removes the stored passphrase (when saved). */
  removePassphrase(index: number): void {
    const entry = this.editing?.clientCertificates[index];
    if (!entry) return;
    entry.encryptedPassphrase = null;
    entry.newPassphrase = '';
  }

  /** Applies and saves; the dialog stays open with the reason if that fails. */
  async save(): Promise<void> {
    const editing = this.editing;
    if (!editing) return;
    this.saving = true;
    this.error = null;
    try {
      const ack = await this.#api.execute({
        type: 'saveSettings',
        settings: $state.snapshot(editing),
      });
      if (ack.type !== 'settingsSaved') throw new Error(`unexpected answer ${ack.type}`);
      this.close();
    } catch (error) {
      this.error = toUserText(error);
    } finally {
      this.saving = false;
    }
  }
}
