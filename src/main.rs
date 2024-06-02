use factorio::{FactorioState, Input};
use ggez::event;
use ggez::graphics::{self, Color};
use ggez::input::keyboard::KeyInput;
use ggez::{Context, GameResult};
use ggez::glam::*;

use std::time::Instant;

mod factorio;

struct MainState {
    pos_x: f32,
    previous: Instant,
    factorio: FactorioState,
    input: Input,
}

impl MainState {
    fn new() -> GameResult<MainState> {
        let s = MainState {
            pos_x: 0.0,
            previous: Instant::now(),
            factorio: FactorioState::new(),
            input: Input::default(),
        };
        Ok(s)
    }
}

impl event::EventHandler<ggez::GameError> for MainState {
    fn update(&mut self, _ctx: &mut Context) -> GameResult {
        self.pos_x = self.pos_x % 800.0 + 1.0;
        self.previous = Instant::now();

        self.factorio.update(self.input.clone());
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let mut canvas = graphics::Canvas::from_frame(
            ctx,
            graphics::Color::from([0.1, 0.2, 0.3, 1.0]),
        );

        let circle = graphics::Mesh::new_circle(
            ctx,
            graphics::DrawMode::fill(),
            Vec2::new(0.0, 0.0),
            100.0,
            2.0,
            Color::WHITE,
        )?;
        canvas.draw(&circle, Vec2::new(self.pos_x, 380.0));

        canvas.finish(ctx)?;
        Ok(())
    }

    fn key_down_event(
        &mut self,
        ctx: &mut Context,
        input: ggez::input::keyboard::KeyInput,
        _repeated: bool,
    ) -> Result<(), ggez::GameError> {
        println!("Key down: {:?}", input);

        match input.keycode.unwrap() {
            
            other_input => {
                println!("unhandled key: {:?}", other_input);
            }
        }

        Ok(())
    }

    fn key_up_event(&mut self, _ctx: &mut Context, input: ggez::input::keyboard::KeyInput) -> Result<(), ggez::GameError> {
        println!("Key up: {:?}", input);
        Ok(())
    }
}

pub fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("super_simple", "ggez");
    let (ctx, event_loop) = cb.build()?;
    let state = MainState::new()?;
    event::run(ctx, event_loop, state)
}
