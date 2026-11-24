// El CHANGELOG del repositorio (inglés y español), empaquetado con la app (sin red ni archivos del vault).
import en from '../../../../CHANGELOG.md?raw';
import es from '../../../../CHANGELOG.es.md?raw';

export const CHANGELOG_RAW: Record<'en' | 'es', string> = { en, es };
