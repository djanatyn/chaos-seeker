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
    cost: u8,
    /// Estimated distance to goal tile.
    heuristic: u8,
}

impl PartialOrd for SearchTile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchTile {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .heuristic
            .cmp(&self.heuristic)
            .then_with(|| self.cost.cmp(&other.cost))
    }
}

impl PathProblem<'_> {
    fn solve(&self) -> anyhow::Result<()> {
        let mut frontier: BinaryHeap<SearchTile> = BinaryHeap::from([SearchTile {
            coordinate: self.starting_tile,
            cost: 0,
            heuristic: self.heuristic(&self.desired_tile),
        }]);
        let mut reached: HashSet<Coordinate> = HashSet::new();
        let mut history: HashMap<Coordinate, Coordinate> = HashMap::new();
        let mut cost: HashMap<Coordinate, u8> = HashMap::new();

        while let Some(current) = frontier.pop() {
            if current.coordinate == self.desired_tile {
                todo!("found path")
            }

            for next in self.neighbors(&current.coordinate) {
                let new_cost =
                    cost[&current.coordinate] + Self::distance_cost(&current.coordinate, &next);
                todo!()
            }
        }

        todo!()
    }

    fn distance_cost(start: &Coordinate, end: &Coordinate) -> u8 {
        start.0.abs_diff(end.0) + end.1.abs_diff(end.1)
    }

    fn heuristic(&self, coordinate: &Coordinate) -> u8 {
        todo!()
    }

    fn neighbors(&self, coordinate: &Coordinate) -> impl Iterator<Item = Coordinate> {
        std::iter::repeat((0, 0)) // TODO
    }
}
