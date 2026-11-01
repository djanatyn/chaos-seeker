use crate::overworld::WorldMap;

type Coordinate = (u8, u8); // (x, y)

#[derive(Debug)]
struct PathScore {
    source: u8,
    destination: u8,
}

#[derive(Debug)]
struct Path {
    postition: Coordinate,
    score: PathScore,
    visited: Vec<Coordinate>,
}

impl Path {
    fn neighbors(&self) -> impl Iterator<Item = Coordinate> {
        std::iter::repeat((0, 0)) // TODO
    }
}

#[derive(Debug)]
struct PathProblem {
    starting_tile: Coordinate,
    desired_tile: Coordinate,
}

impl PathProblem {
    fn solve(&self, map: WorldMap) -> anyhow::Result<Path> {
        todo!()
    }

    fn score(&self, coord: &Coordinate) -> anyhow::Result<PathScore> {
        todo!()
    }
}
