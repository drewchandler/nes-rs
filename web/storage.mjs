export class SaveStorage {
  constructor(indexedDB = globalThis.indexedDB) {
    this.ready = new Promise((resolve, reject) => {
      if (!indexedDB) { reject(new Error('Browser storage is unavailable.')); return; }
      const request = indexedDB.open('nes-rs', 1);
      request.onupgradeneeded = () => { request.result.createObjectStore('saves'); };
      request.onsuccess = () => {
        const db = request.result;
        db.onversionchange = () => db.close();
        resolve(db);
      };
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(new Error('Browser storage is blocked by another tab.'));
    });
    // Storage can be denied in private/embedded browsers; playing still works.
    this.ready.catch(() => {});
  }
  async get(kind, id) {
    const db = await this.ready;
    return new Promise((resolve, reject) => {
      const transaction = db.transaction('saves', 'readonly');
      const request = transaction.objectStore('saves').get(`${kind}:${id}`);
      request.onsuccess = () => resolve(request.result ? new Uint8Array(request.result) : null);
      request.onerror = () => reject(request.error);
      transaction.onabort = () => reject(transaction.error);
    });
  }
  async put(kind, id, bytes) {
    const db = await this.ready;
    return new Promise((resolve, reject) => {
      const transaction = db.transaction('saves', 'readwrite');
      transaction.objectStore('saves').put(bytes.slice(), `${kind}:${id}`);
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
  }
}
export function equalBytes(first, second) {
  return first?.length === second?.length && first.every((byte, i) => byte === second[i]);
}
