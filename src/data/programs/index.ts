import type { Program } from '../program';

/**
 * Every `practice-program/v1` file in this folder. A missing or malformed file is skipped, so the app
 * still builds and runs (as the plain trainer) without a program.
 */
const files = import.meta.glob('./*.json', { eager: true, import: 'default' }) as Record<string, unknown>;

export const PROGRAMS: Program[] = Object.values(files).filter((p): p is Program => {
  const x = p as Partial<Program> | null;
  return !!x && x.schema === 'practice-program/v1' && !!x.program && !!x.exercises && Array.isArray(x.sessions);
});
