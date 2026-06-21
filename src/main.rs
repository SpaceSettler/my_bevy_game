use std::collections::VecDeque;
use oorandom::Rand32;
use getrandom;
use ggez::{
    context::{ContextFields, HasMut},
    event, graphics,
    input::keyboard::KeyInput,
    Context, GameResult,
};
use winit::keyboard::{Key, NamedKey};

//collection of constants for ease of use on sizing.
const GRID_SIZE: (i16, i16) = (16, 8);

const GRID_CELL_SIZE: (i16, i16) = (32, 32);

const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);

//make sure things run orderly at a constant rate
const DESIRED_FPS: u32 = 64;

//first struct is just essetnially a coordiante with function to generate coords, and convert.
#[derive(Clone, Copy, PartialEq)]
struct GridPos {
    x: i16,
    y: i16,
}

impl GridPos {

    pub fn new(x: i16, y: i16) -> Self {
        GridPos { x, y }
    }

    pub fn random(rng: &mut Rand32, max_x: i16, dif_y: i16) -> Self {
        (
            rng.rand_range(0..(max_x as u32)) as i16,
            dif_y,
        )
            .into()
    }
}

impl From<GridPos> for graphics::Rect {
    fn from(pos: GridPos) -> Self {
        graphics::Rect::new_i32(
            pos.x as i32 * GRID_CELL_SIZE.0 as i32,
            pos.y as i32 * GRID_CELL_SIZE.1 as i32,
            GRID_CELL_SIZE.0 as i32,
            GRID_CELL_SIZE.1 as i32,
        )
    }
}

impl From<(i16, i16)> for GridPos {
    fn from(pos: (i16, i16)) -> Self {
        GridPos { x: pos.0, y: pos.1 }
    }
}

//enum for directions able to mov,e and translate from raw keypress into usable names
#[derive(Copy, Clone, PartialEq)]
enum Move {
    Left,
    Right,
    None,
}

impl Move {

    pub fn from_key(key: &Key) -> Option<Move> {
        match key {
            Key::Named(NamedKey::ArrowLeft) => Some(Move::Left),
            Key::Named(NamedKey::ArrowRight) => Some(Move::Right),
            _ => None,
        }
    }
}

//obstable to ovoid by player. Made of a position on the grid, a vector of itself, and other variables to check conditions
struct Obstacle {
    pos: GridPos,
    body: VecDeque<GridPos>,
    counter: i32,
    drop_time: i32,
    end_reached: bool,
    increment: i16,
}

impl Obstacle {
    pub fn new(pos: GridPos, timer: i32) -> Self {
        //create the vector deque and add the first part of the line
        let mut body = VecDeque::new();
        body.push_back(GridPos::new(pos.x, pos.y).into());

        Obstacle {
            pos,
            body,
            counter: 0,
            drop_time: timer,
            end_reached: false,
            increment: 1
        }
    }

    fn update(&mut self) {
        //a counter runs each frame and when it hits a threshold (drop_time) it moves the line down closer to the player, then resets
        //and starts over.
        if self.counter == self.drop_time {
             self.body.push_back(GridPos::new(self.pos.x, self.pos.y + self.increment).into());
             self.counter = 0;
             self.increment += 1;
        } else {
            self.counter += 1;
        }
        //once the lines reaches the bottom of screen we can remove it and start a new one
        if self.pos.y + self.increment > 8 {
            self.increment = 1;
            self.end_reached = true;
        }
    }

    fn draw(&mut self, canvas: &mut graphics::Canvas) {
        //draws each piece of the obstacle by using the from helper function defined in GridPos to convert to usable rectangle data for ggez drawing
        for seg in &self.body {
            //redraws the whole line in the background color, effectively erasing it, if the line has reached the bottom of screen. Otherwise draws
            //the line normally
            if self.end_reached {
                canvas.draw(
                    &graphics::Quad,
                    graphics::DrawParam::new()
                        .dest_rect((*seg).into())
                        .color([0.0, 1.0, 0.0, 1.0]),
                );
                self.end_reached = false;    
            } else {
                canvas.draw(
                    &graphics::Quad,
                    graphics::DrawParam::new()
                        .dest_rect((*seg).into())
                        .color([0.3, 0.3, 0.0, 1.0]),
                );
            }
        }
    }
}

//the player also has a GridPos but this time only tracks their movement and whether or not they hit the obstacle
struct Player {
    pos: GridPos,
    dir: Move,
    is_dead: bool,
}

impl Player {
    pub fn new(pos: GridPos) -> Self {
        Player {
            pos,
            dir: Move::None,
            is_dead: false,
        }
    }

    //checks for collision between the bottom of the line and the player
     fn hits(&self, obstacle: &Obstacle) -> bool {
        self.pos.x == obstacle.pos.x && self.pos.y == obstacle.pos.y + obstacle.increment
    }

    fn update(&mut self, obstacle: &Obstacle) {

        //if the key pressed is aligned with a directional movement defined in the move enum then we check and see if the player can move the
        //way they want to and then move them there if posible.
        match self.dir {
            Move::Left => {
                if self.pos.x > 0 {
                    self.pos.x-=1;
                }
                self.dir = Move::None;
            },
            Move::Right => {
                if self.pos.x < 15 {
                    self.pos.x+=1;   
                }
                self.dir = Move::None;
            },
            Move::None => (),
        }

        //did they hit the obstacle? If yes then they are dead.
        if self.hits(obstacle) {
           self.is_dead = true;
        } 
    }

    //same draw as the obstacle, but no deque to loop through
    fn draw(&self, canvas: &mut graphics::Canvas) {
        let color = [0.0, 0.0, 1.0, 1.0];
        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.pos.into())
                .color(color),
        );
    }
}


//the game state kinda holds most things in the game, so it has the player and the obstacle, plus some variables for checking things, and
//difficulty manipulation
struct GameState {
    player: Player,
    obstacle: Obstacle,
    gameover: bool,
    rng: Rand32,
    score: i32,
    dif_multiplier: i16,
    timer: i32,
}

impl GameState {
    pub fn new() -> Self {
        //set the player in the middle of the screen
        let player_pos = (8, 7).into();
        //set the starting hight to 7
        let dif_multiplier = 7;
        //use random librarys to generate seeds to randomize the placement of the lines
        let mut seed: [u8; 8] = [0; 8];
        getrandom::fill(&mut seed[..]).expect("Could not create RNG seed");
        let mut rng = Rand32::new(u64::from_ne_bytes(seed));
        //start line at a random spot as defined by the seed
        let obstacle_pos = GridPos::random(&mut rng, GRID_SIZE.0, GRID_SIZE.1-dif_multiplier);

        GameState {
            player: Player::new(player_pos),
            obstacle: Obstacle::new(obstacle_pos, 8),
            gameover: false,
            rng,
            score: 0,
            dif_multiplier,
            timer: 8,
        }
    }
}

impl event::EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        //only update at the desired rate
        while ctx.time.check_update_time(DESIRED_FPS) {
            if !self.gameover {
                //call update in player with reference to obstacle so we can see if they are hit by it. If they are end the game
                self.player.update(&self.obstacle);
                if self.player.is_dead {
                    self.gameover = true;
                }
                self.obstacle.update();
                if self.obstacle.end_reached {
                    //if the obstacle reaches the end make a new one somewhere else randomly
                    let new_obstacle_pos = GridPos::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1-self.dif_multiplier);
                    self.obstacle = Obstacle::new(new_obstacle_pos, self.timer);
                    //increase the score, and every ten times either lower the height of lines, or increase their speed
                    self.score += 1;
                    if self.score % 10 == 0 && self.timer != 2 && self.dif_multiplier != 3 {
                        if self.score % 20 == 0 {
                            self.timer -= 2;
                        } else {
                            self.dif_multiplier -= 1;
                        }
                    }
                }
            }
        }

        Ok(())
    }
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        //draw the background on solid color, the same color will be used to erase old lines
        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.0, 1.0, 0.0, 1.0]));

        //draw calls for both player and obstacle
        self.player.draw(&mut canvas);
        self.obstacle.draw(&mut canvas);
      
        //canvas finish will actaully draw everything it has queued up
        canvas.finish(ctx)?;
        
        //yeilds control and gives back to system because of fps implementation
        ggez::timer::yield_now();
        Ok(())
    }
    
    //if a key is pressed get its data, and attempt to translate via enum function.
    fn key_down_event(&mut self, ctx: &mut Context, input: KeyInput, _repeat: bool) -> GameResult {
        if let Some(dir) = Move::from_key(&input.event.logical_key) {
            self.player.dir = dir;
        } else if input.event.logical_key == Key::Named(NamedKey::Escape) {
            HasMut::<ContextFields>::retrieve_mut(ctx).quit_requested = true;
        }
        Ok(())
    }
}

fn main() -> GameResult {
    //generic setup
    let (ctx, events_loop) = ggez::ContextBuilder::new("Line Dodge", "SpaceSettler")
        .window_setup(ggez::conf::WindowSetup::default().title("LineDodge!"))
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        .build()?;
        
    let state = GameState::new();
    //actually runs the whole thing by calling everything built and the running in event loop
    event::run(ctx, events_loop, state)
}