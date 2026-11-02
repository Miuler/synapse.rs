import { describe, expect, it } from 'bun:test';
import { readFileSync } from 'fs';
import { createSynapseApp } from './app';
import { getObsidianBundle } from './bridge';
import type { VaultRepository } from '@shared/repositories';

/**
 * Mapa de compatibilidad con el API oficial de Obsidian (paquete `obsidian`, solo tipos).
 * Lista los valores exportados por el módulo `obsidian` que la fachada todavía no ofrece a los
 * plugins. La instantánea hace visible cualquier cambio: si se implementa uno, sale de la lista;
 * si se rompe una exportación existente, entra.
 */
function officialRuntimeExports(): string[] {
  const dts = readFileSync(require.resolve('obsidian/obsidian.d.ts'), 'utf8');
  const names = new Set<string>();
  for (const m of dts.matchAll(/^export (?:declare )?(?:abstract )?(?:class|function|const|let|enum) (\w+)/gm)) {
    names.add(m[1]);
  }
  return [...names].sort();
}

describe('Compatibilidad con el API oficial de Obsidian', () => {
  it('exportaciones del módulo obsidian que la fachada no ofrece', () => {
    const repo = new Proxy({}, { get: () => async () => null }) as unknown as VaultRepository;
    const bundle = getObsidianBundle(createSynapseApp(repo));
    const official = officialRuntimeExports();
    const missing = official.filter((name) => !(name in bundle));
    const covered = official.length - missing.length;
    expect({ total: official.length, covered, missing }).toMatchSnapshot();
  });
});
