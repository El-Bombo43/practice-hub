/** Signed offset in ms with a real minus sign: +12, −3, 0. */
export const fmtOffset = (ms: number): string => {
  const r = Math.round(ms);
  return r === 0 ? '0' : r > 0 ? `+${r}` : `−${Math.abs(r)}`;
};
