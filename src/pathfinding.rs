use crate::overworld::WorldMap;

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    path::PathBuf,
};

type Coordinate = (u8, u8); // (x, y)

#[derive(Debug)]
struct PathProblem<'a> {
    starting_tile: Coordinate,
    goal_tile: Coordinate,
    map: &'a WorldMap,
}

#[derive(Debug, PartialEq, Eq)]
struct SearchTile {
    /// Coordinate of tile.
    coordinate: Coordinate,
    /// Actual distance from starting tile.
    cost: u32,
    /// Cost + estimated distance to goal tile.
    priority: u32,
}

impl PartialOrd for SearchTile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchTile {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.cost.cmp(&other.cost))
    }
}

impl PathProblem<'_> {
    fn solve(&self) -> anyhow::Result<Vec<Coordinate>> {
        let mut frontier: BinaryHeap<SearchTile> = BinaryHeap::from([SearchTile {
            coordinate: self.starting_tile,
            cost: 0,
            priority: self.heuristic(&self.starting_tile),
        }]);
        let mut history: HashMap<Coordinate, Coordinate> = HashMap::new();
        let mut cost: HashMap<Coordinate, u32> = HashMap::from([(self.starting_tile, 0)]);

        while let Some(current) = frontier.pop() {
            if current.cost != cost[&current.coordinate] {
                continue;
            }

            if current.coordinate == self.goal_tile {
                let mut path = vec![self.goal_tile];
                let mut current = self.goal_tile;

                while current != self.starting_tile {
                    current = *history
                        .get(&current)
                        .ok_or_else(|| anyhow::anyhow!("broken path"))?;
                    path.push(current);
                }

                path.reverse();
                return Ok(path);
            }

            for next in self.neighbors(&current.coordinate) {
                let new_cost = current.cost + Self::distance_cost(&current.coordinate, &next);
                if cost
                    .get(&next)
                    .is_none_or(|known_cost| new_cost < *known_cost)
                {
                    cost.insert(next, new_cost);
                    history.insert(next, current.coordinate);

                    let priority = new_cost + self.heuristic(&next);
                    frontier.push(SearchTile {
                        coordinate: next,
                        cost: new_cost,
                        priority,
                    });
                }
            }
        }

        Err(anyhow::anyhow!("no path found"))
    }

    fn distance_cost(start: &Coordinate, end: &Coordinate) -> u32 {
        u32::from(start.0.abs_diff(end.0)) + u32::from(start.1.abs_diff(end.1))
    }

    fn heuristic(&self, coordinate: &Coordinate) -> u32 {
        Self::distance_cost(coordinate, &self.goal_tile)
    }

    fn neighbors(&self, coordinate: &Coordinate) -> impl Iterator<Item = Coordinate> {
        const DIRECTIONS: [(isize, isize); 4] = [
            (1, 0),  // right
            (-1, 0), // left
            (0, -1), // up
            (0, 1),  // down
        ];

        let x = coordinate.0 as isize;
        let y = coordinate.1 as isize;

        DIRECTIONS.into_iter().filter_map(move |(dx, dy)| {
            let nx = x + dx;
            let ny = y + dy;

            if !(0..256).contains(&nx) || !(0..256).contains(&ny) {
                return None;
            }

            let index: (usize, usize) = (nx as usize, ny as usize);
            let neighbor: (u8, u8) = (nx as u8, ny as u8);
            self.map
                .get(index)
                .and_then(|tile| self.map.lookup(tile))
                .filter(|properties| properties.walk_allowed())
                .map(|_| neighbor)
        })
    }
}

#[test]
fn test_pathfinding() -> anyhow::Result<()> {
    let rom = std::fs::read("ff1.nes")?;
    let map = WorldMap::parse(&rom)?;

    let problem = PathProblem {
        starting_tile: (153, 167),
        goal_tile: (125, 121),
        map: &map,
    };

    let solution = problem.solve()?;
    insta::assert_debug_snapshot!(solution);

    map.map
        .save_pathfinding_image(solution, &PathBuf::from("pathfinding.png"), &map.properties)?;

    Ok(())
}
