use crate::overworld::WorldMap;

use std::collections::{HashMap, HashSet, VecDeque};

type Coordinate = (u8, u8); // (x, y)

#[derive(Debug)]
struct PathProblem<'a> {
    starting_tile: Coordinate,
    desired_tile: Coordinate,
    map: &'a WorldMap,
}

impl PathProblem<'_> {
    fn solve(&self) -> anyhow::Result<()> {
        let mut frontier: VecDeque<Coordinate> = VecDeque::new();
        let mut reached: HashSet<Coordinate> = HashSet::new();
        let mut history: HashMap<Coordinate, Coordinate> = HashMap::new();

        todo!()
    }

    fn neighbors(&self, coordinate: &Coordinate) -> impl Iterator<Item = Coordinate> {
        std::iter::repeat((0, 0)) // TODO
    }
}
