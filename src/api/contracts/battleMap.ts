import * as v from 'valibot';

const cell = v.object({
  x: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(127)),
  y: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(127)),
});
export const BattleMapSchema = v.pipe(
  v.object({
    width: v.pipe(v.number(), v.integer(), v.minValue(1), v.maxValue(128)),
    height: v.pipe(v.number(), v.integer(), v.minValue(1), v.maxValue(128)),
    blocked: v.pipe(v.array(cell), v.maxLength(16384)),
    exits: v.pipe(v.array(cell), v.maxLength(8)),
  }),
  v.check(
    (map) =>
      map.blocked.length <= map.width * map.height &&
      [...map.blocked, ...map.exits].every((cell) => cell.x < map.width && cell.y < map.height) &&
      map.exits.every(
        (exit) => !map.blocked.some((cell) => cell.x === exit.x && cell.y === exit.y),
      ),
  ),
);
export type BattleMap = v.InferOutput<typeof BattleMapSchema>;
