export type Lane = 'L' | 'R' | 'down' | 'up';
/** ghost = hand moves, no hit expected */
export type Slot = { play: boolean; lane?: Lane; accent?: boolean; ghost?: boolean };
export type Exercise = {
  id: string;
  name: string;
  instrument: 'guitar' | 'pads';
  kind: 'picking' | 'strum' | 'rudiment';
  beatsPerBar: number;
  subdivision: 2 | 3 | 4 | 6; // notes per beat
  pattern: Slot[]; // one bar; length = beatsPerBar * subdivision
  startBpm: number;
  ramp: { stepBpm: number; cleanRepsToAdvance: number };
  windowMs: number; // default 15
  minutes: number; // default length in a routine
};
