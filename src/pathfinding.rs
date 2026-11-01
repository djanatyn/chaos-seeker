use crate::overworld::WorldMap;

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

type Coordinate = (u8, u8); // (x, y)

#[derive(Debug)]
struct PathProblem<'a> {
    starting_tile: Coordinate,
    desired_tile: Coordinate,
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
            if current.coordinate == self.desired_tile {
                let mut path = vec![self.desired_tile];
                let mut current = self.desired_tile;

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
                let new_cost =
                    cost[&current.coordinate] + Self::distance_cost(&current.coordinate, &next);
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
        (start.0.abs_diff(end.0) + start.1.abs_diff(end.1)) as u32
    }

    fn heuristic(&self, coordinate: &Coordinate) -> u32 {
        todo!()
    }

    fn neighbors(&self, coordinate: &Coordinate) -> impl Iterator<Item = Coordinate> {
        std::iter::repeat((0, 0)) // TODO
    }
}
