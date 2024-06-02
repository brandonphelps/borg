use std::f32::consts::PI;

// todo: move this to some generic layer that allows
// for serde like macro key binding action.
#[derive(Default, Debug, Copy, Clone)]
pub struct Input {
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub right_pressed: bool,
    pub left_pressed: bool,
}

// a Facing direction to determine
// what orientation a player is moving.
#[derive(Debug, Copy, Clone)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    // returns the components of a direction
    // i.e north is 0.0, 1.0
    //     east is 1.0, 0.0
    //     north east is then, cos(45 degrees), sin(45 degreess)
    fn angle(&self) -> (f32, f32) {
        match self {
            Direction::North => (0.0, 1.0),
            Direction::NorthEast => ((PI / 4.0).cos() as f32, (PI / 4.0).sin() as f32),
            Direction::East => (1.0, 0.0),
            Direction::SouthEast => ((7.0 * PI / 4.0).cos() as f32, (7.0 * PI / 4.0).sin() as f32),
            Direction::South => (0.0, -1.0),
            Direction::SouthWest => ((5.0 * PI / 4.0).cos() as f32, (5.0 * PI / 4.0).sin() as f32),
            Direction::West => (-1.0, 0.0),
            Direction::NorthWest => ((3.0 * PI / 4.0).cos() as f32, (3.0 * PI / 4.0).sin() as f32),
        }
    }
}

//    /|
// z / | y
//  /  |
// -----
//   x

// f(x) =
//

struct Player {
    // todo: also keep track of a "chunk" region
    // so that positions don't get super large. 
    position: (f32, f32),
    direction: Direction,
}

enum TileMat {
    Ground,
    Water,
}

struct Tile {
    ground: TileMat,
}

pub struct FactorioState {
    player: Player,
    tiles: Vec<Tile>,
}

impl FactorioState {
    pub fn new() -> Self {
        Self {
            player: Player {
                position: (0.0, 0.0),
                direction: Direction::North,
            },
            tiles: vec![],
        }
    }

    pub fn update(&mut self, input: Input) {
        let direction = if input.up_pressed {
            if input.left_pressed && !input.right_pressed {
                Some(Direction::NorthWest)
            } else if !input.left_pressed && input.right_pressed {
                Some(Direction::NorthEast)
            } else {
                Some(Direction::North)
            }
        } else if input.down_pressed {
            if input.left_pressed && !input.right_pressed {
                Some(Direction::SouthWest)
            } else if !input.left_pressed && input.right_pressed {
                Some(Direction::SouthEast)
            } else {
                Some(Direction::South)
            }
        } else if input.left_pressed {
            Some(Direction::West)
        } else if input.right_pressed {
            Some(Direction::East) 
        } else {
            None
        };

        if let Some(dir) = direction {
            println!("input: {:?}, dir: {:?}", input, dir);
            self.player.direction = dir;
            let dir = self.player.direction.angle();
            self.player.position = (
                self.player.position.0 + 1.0 * dir.0,
                self.player.position.1 + 1.0 * dir.1,
            );
            println!("{:?}", self.player.position);
        }
    }
}
