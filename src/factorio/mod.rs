
// todo: move this to some generic layer that allows
// for serde like macro key binding action. 
#[derive(Default, Copy, Clone)]
pub struct Input {
    up_pressed: bool,
    down_pressed: bool,
    right_pressed: bool,
    left_pressed: bool
}

struct Player {
    position: (f32, f32),
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
            player: Player { position: (0.0, 0.0) },
            tiles: vec![]
        }
    }

    pub fn update(&mut self, input: Input) {
        
    }
}


