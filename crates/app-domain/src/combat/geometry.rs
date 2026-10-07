use super::types::Position;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleMap {
    pub width: u16,
    pub height: u16,
    pub blocked: Vec<Position>,
    pub exits: Vec<Position>,
}
impl BattleMap {
    pub fn valid(&self) -> bool {
        (1..=128).contains(&self.width)
            && (1..=128).contains(&self.height)
            && self.blocked.len() <= usize::from(self.width) * usize::from(self.height)
            && self.blocked.iter().all(|cell| self.inside(*cell))
            && self.exits.len() <= 8
            && self.exits.iter().all(|cell| self.walkable(*cell))
    }
    pub fn inside(&self, p: Position) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < i32::from(self.width) && p.y < i32::from(self.height)
    }
    pub fn walkable(&self, p: Position) -> bool {
        self.inside(p) && !self.blocked.contains(&p)
    }
    pub fn neighbors(&self, p: Position) -> Vec<Position> {
        (-1..=1)
            .flat_map(|dx| (-1..=1).map(move |dy| (dx, dy)))
            .filter(|(dx, dy)| *dx != 0 || *dy != 0)
            .filter_map(|(dx, dy)| {
                let next = Position {
                    x: p.x.saturating_add(dx),
                    y: p.y.saturating_add(dy),
                };
                if self.walkable(next)
                    && (dx == 0
                        || dy == 0
                        || self.walkable(Position {
                            x: p.x + dx,
                            y: p.y,
                        }) && self.walkable(Position {
                            x: p.x,
                            y: p.y + dy,
                        }))
                {
                    Some(next)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn path(
        &self,
        from: Position,
        to: Position,
        occupied: &HashSet<Position>,
    ) -> Option<Vec<Position>> {
        if !self.valid() || !self.walkable(from) || !self.walkable(to) || occupied.contains(&to) {
            return None;
        }
        if from == to {
            return Some(vec![]);
        }
        let mut parents = HashMap::new();
        let mut seen = HashSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(cell) = queue.pop_front() {
            for next in self.neighbors(cell) {
                if occupied.contains(&next) || !seen.insert(next) {
                    continue;
                }
                parents.insert(next, cell);
                if next == to {
                    let mut result = vec![to];
                    let mut cursor = to;
                    while parents[&cursor] != from {
                        cursor = parents[&cursor];
                        result.push(cursor);
                    }
                    result.reverse();
                    return Some(result);
                }
                queue.push_back(next);
            }
        }
        None
    }
    pub fn clear_line(&self, from: Position, to: Position) -> bool {
        if !self.walkable(from) || !self.walkable(to) {
            return false;
        }
        let (mut x, mut y) = (from.x, from.y);
        let dx = (to.x - x).abs();
        let dy = -(to.y - y).abs();
        let sx = if x < to.x { 1 } else { -1 };
        let sy = if y < to.y { 1 } else { -1 };
        let mut error = dx + dy;
        while x != to.x || y != to.y {
            let twice = error * 2;
            let old = Position { x, y };
            if twice >= dy {
                error += dy;
                x += sx;
            }
            if twice <= dx {
                error += dx;
                y += sy;
            }
            let next = Position { x, y };
            if !self.walkable(next)
                || x != old.x
                    && y != old.y
                    && (!self.walkable(Position { x, y: old.y })
                        || !self.walkable(Position { x: old.x, y }))
            {
                return false;
            }
        }
        true
    }
}
pub fn bell_archive_map() -> BattleMap {
    BattleMap {
        width: 12,
        height: 8,
        blocked: (0..8)
            .filter(|y| *y != 3)
            .map(|y| Position { x: 6, y })
            .collect(),
        exits: vec![Position { x: 1, y: 3 }],
    }
}
