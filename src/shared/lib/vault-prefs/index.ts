// Preferencias de interfaz recordadas por bóveda (p. ej. si el explorador o el panel del agente
// están fijados). Se guardan en localStorage con la ruta de la bóveda en la clave; si una bóveda
// aún no tiene valor propio, se usa el último global (el de antes de guardarse por bóveda).

const keyFor = (key: string, vault: string | null | undefined) => (vault ? `${key}::${vault}` : key);

export function loadVaultFlag(key: string, vault: string | null | undefined, fallback: boolean): boolean {
  try {
    const own = localStorage.getItem(keyFor(key, vault));
    const value = own ?? localStorage.getItem(key);
    return value === null ? fallback : value !== '0';
  } catch {
    return fallback;
  }
}

export function saveVaultFlag(key: string, vault: string | null | undefined, value: boolean): void {
  try {
    localStorage.setItem(keyFor(key, vault), value ? '1' : '0');
  } catch {
    /* sin almacenamiento */
  }
}
