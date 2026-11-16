const SKIP_ARCHIVED_KEY = 'forgepulse.skipArchived';

export function loadSkipArchived(storage: Pick<Storage, 'getItem'> | undefined = globalThis.localStorage): boolean {
  try {
    return storage?.getItem(SKIP_ARCHIVED_KEY) === 'true';
  } catch {
    return false;
  }
}

export function saveSkipArchived(value: boolean, storage: Pick<Storage, 'setItem'> | undefined = globalThis.localStorage): void {
  try {
    storage?.setItem(SKIP_ARCHIVED_KEY, String(value));
  } catch {
    // Private windows or blocked storage: the choice just isn't remembered.
  }
}
