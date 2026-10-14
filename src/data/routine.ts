export type RoutineItem = { uid: string; exerciseId: string; minutes: number; burst?: boolean };
export type Routine = { items: RoutineItem[]; step: number; stepDate: string };

let counter = 0;
export const newUid = (): string => `r${Date.now().toString(36)}${(counter++).toString(36)}`;

const item = (exerciseId: string, minutes: number, burst = false): RoutineItem => ({
  uid: newUid(), exerciseId, minutes, ...(burst ? { burst: true } : {}),
});

export const defaultRoutine = (): Routine => ({
  items: [
    item('tremolo', 3),
    item('pads-paradiddle', 4),
    item('chromatic', 5),
    item('strum-16th-accents', 4),
    item('pads-gallop', 3),
    item('chromatic', 2, true),
  ],
  step: 0,
  stepDate: '',
});

export const totalMinutes = (r: Routine): number => r.items.reduce((s, i) => s + i.minutes, 0);
