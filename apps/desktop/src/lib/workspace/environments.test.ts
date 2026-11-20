import { describe, expect, it } from 'vitest';

import { memoryStorage, shop } from '../../test/fake-api';
import { EnvironmentChoice } from './environments.svelte';

describe('EnvironmentChoice', () => {
  it('remembers the chosen environment per collection', () => {
    const storage = memoryStorage();
    const choice = new EnvironmentChoice(storage);
    expect(choice.of(shop())).toBeNull();

    choice.choose('c1', 'e1');
    expect(choice.of(shop())?.name).toBe('Dev');
    expect(new EnvironmentChoice(storage).of(shop())?.name).toBe('Dev');

    choice.choose('c1', null);
    expect(choice.of(shop())).toBeNull();
    expect(storage.data.get('activeEnvironments')).toBe('{}');
  });

  it('ignores choices of environments that no longer exist', () => {
    const choice = new EnvironmentChoice(
      memoryStorage({ activeEnvironments: JSON.stringify({ c1: 'gone' }) }),
    );
    expect(choice.of(shop())).toBeNull();
    expect(choice.of(undefined)).toBeNull();
  });

  it('starts empty with unusable storage content', () => {
    const choice = new EnvironmentChoice(memoryStorage({ activeEnvironments: '["e1"]' }));
    expect(choice.of(shop())).toBeNull();
  });
});
