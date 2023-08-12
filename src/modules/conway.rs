/*!
The `Conway` module outputs a signal based on a Conway's Game of Life
simulation.

## Inputs
0. Whether to reset the simulation, any non-zero value for yes

## Outputs
0. The simulation signal for each pixel

## Knobs
0. The signal to output for dead pixels
1. The signal to output for newly alive pixels
2. The signal to output for alive pixels
3. The signal to output for newly dead pixels

*/

use bevy::{prelude::*, ecs::system::EntityCommands, sprite::Mesh2dHandle};

use rand::Rng;
use serde::Deserialize;

use crate::{StepType, modules::{Module, ModuleComponent, ModuleTextComponent, ModuleImageComponent, ModuleMeshComponent, component_video_out::ComponentVideoOut}};

fn default_half() -> f64 {
    0.5
}

#[derive(Default, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    #[default]
    Dead,
    NewlyAlive,
    Alive,
    NewlyDead,
}

#[derive(Default, Deserialize, Debug, Clone)]
pub struct Conway {
    #[serde(skip)]
    id: Option<usize>,
    #[serde(default)]
    name: Option<String>,

    #[serde(skip)]
    component: Option<Entity>,
    #[serde(skip)]
    children: Vec<Entity>,

    #[serde(default)]
    seed: String,
    #[serde(default = "default_half")]
    density: f64,
    #[serde(skip)]
    rng: Option<rand::rngs::StdRng>,
    #[serde(skip)]
    grid: Option<[[Cell; ComponentVideoOut::WIDTH]; ComponentVideoOut::HEIGHT]>,
    #[serde(skip)]
    scan: usize,

    knobs: [f32; 4],
}
impl Conway {
    fn init_grid(&mut self) {
        let seed = self.seed.chars()
            .map(|c| c as u8)
            .chain([0u8; 32])
            .collect::<Vec<u8>>();
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed(seed[..32].try_into().unwrap());
        let mut grid = [[Cell::Dead; ComponentVideoOut::WIDTH]; ComponentVideoOut::HEIGHT];
        for row in &mut grid {
            for col in row {
                if rng.gen_bool(self.density) {
                    *col = Cell::Alive;
                }
            }
        }
        self.rng = Some(rng);
        self.grid = Some(grid);
    }
}
#[typetag::deserialize]
impl Module for Conway {
    fn init(&mut self, id: usize, mut ec: EntityCommands, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, _materials: &mut ResMut<Assets<ColorMaterial>>, ts: TextStyle) {
        self.id = Some(id);
        ec.with_children(|parent| {
            let mut component = parent.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Relative,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                },
                ModuleComponent,
            ));
            component.with_children(|parent| {
                let name = match &self.name {
                    Some(name) => format!("{name}\n"),
                    None => format!("M{id} Conway\n"),
                };
                self.children.push(
                    parent.spawn((
                        TextBundle::from_sections([
                            TextSection::new(name, ts.clone()),
                            TextSection::new(format!("Seed: {}\n", self.seed), ts.clone()),
                            TextSection::new(format!("Density: {}\n", self.density), ts.clone()),
                            TextSection::new("K0\n", ts.clone()),
                            TextSection::new("K1\n", ts.clone()),
                            TextSection::new("K2\n", ts.clone()),
                            TextSection::new("K3\n", ts),
                        ]),
                        ModuleTextComponent,
                    )).id()
                );
            });
            self.component = Some(component.id());
        });

        if self.grid.is_none() {
            self.init_grid();
        }
    }
    fn exit(&mut self) {
        self.id = None;
        self.component = None;
        self.children = vec![];

        self.rng = None;
        self.grid = None;
        self.scan = 0;
    }

    fn id(&self) -> Option<usize> {
        self.id
    }
    fn name(&self) -> Option<String> {
        self.name.clone()
    }
    fn component(&self) -> Option<Entity> {
        self.component
    }

    fn inputs(&self) -> usize {
        1
    }
    fn outputs(&self) -> usize {
        1
    }
    fn knobs(&self) -> usize {
        self.knobs.len()
    }

    fn get_knobs(&self) -> Vec<f32> {
        self.knobs.to_vec()
    }
    fn set_knob(&mut self, i: usize, val: f32) {
        self.knobs[i] = val;
    }

    fn step(&mut self, _time: f64, _st: StepType, ins: &[f32]) -> Vec<f32> {
        let reset = ins[0];
        if !reset.is_nan() && reset != 0.0 {
            self.rng = None;
            self.grid = None;
            self.scan = 0;
        }

        if self.grid.is_none() {
            self.init_grid();
        }

        let x = self.scan % ComponentVideoOut::WIDTH;
        let y = self.scan / ComponentVideoOut::WIDTH;

        let out = match self.grid.unwrap()[y][x] {
            Cell::Dead => vec![self.knobs[0]],
            Cell::NewlyAlive => vec![self.knobs[1]],
            Cell::Alive => vec![self.knobs[2]],
            Cell::NewlyDead => vec![self.knobs[3]],
        };

        self.scan += 1;
        self.scan %= ComponentVideoOut::WIDTH * ComponentVideoOut::HEIGHT;

        if self.scan == 0 {
            let mut old_grid = self.grid.take().unwrap();
            let mut grid = [[Cell::Dead; ComponentVideoOut::WIDTH]; ComponentVideoOut::HEIGHT];
            for j in 0..ComponentVideoOut::HEIGHT {
                for i in 0..ComponentVideoOut::WIDTH {
                    if old_grid[j][i] == Cell::NewlyAlive {
                        old_grid[j][i] = Cell::Alive;
                    } else if old_grid[j][i] == Cell::NewlyDead {
                        old_grid[j][i] = Cell::Dead;
                    }

                    match get_neighbors(&old_grid, i, j) {
                        2 | 3 if old_grid[j][i] == Cell::Alive => {
                            grid[j][i] = Cell::Alive;
                        },
                        3 if old_grid[j][i] == Cell::Dead => {
                            grid[j][i] = Cell::NewlyAlive;
                        },
                        _ if old_grid[j][i] == Cell::Alive => {
                            grid[j][i] = Cell::NewlyDead;
                        },
                        _ => {
                            grid[j][i] = Cell::Dead;
                        },
                    }
                }
            }
            self.grid = Some(grid);
        }

        out
    }
    fn render(&mut self, _images: &mut ResMut<Assets<Image>>, _meshes: &mut ResMut<Assets<Mesh>>, q_text: &mut Query<&mut Text, With<ModuleTextComponent>>, _q_image: &mut Query<&mut UiImage, With<ModuleImageComponent>>, _q_mesh: &mut Query<&mut Mesh2dHandle, With<ModuleMeshComponent>>) {
        if let Some(component) = self.children.get(0) {
            if let Ok(mut text) = q_text.get_mut(*component) {
                text.sections[3].value = format!("K0 Dead: {}\n", self.knobs[0]);
                text.sections[4].value = format!("K1 Newly Alive: {}\n", self.knobs[1]);
                text.sections[5].value = format!("K2 Alive: {}\n", self.knobs[2]);
                text.sections[6].value = format!("K3 Newly Dead: {}\n", self.knobs[3]);
            }
        }
    }
}

fn get_neighbors(grid: &[[Cell; ComponentVideoOut::WIDTH]; ComponentVideoOut::HEIGHT], x: usize, y: usize) -> usize {
    let mut neighbors = 0;

    let xm = if x > 0 {
        x - 1
    } else {
        ComponentVideoOut::WIDTH - 1
    };
    let ym = if y > 0 {
        y - 1
    } else {
        ComponentVideoOut::HEIGHT - 1
    };

    let xp = if x < ComponentVideoOut::WIDTH - 1 {
        x + 1
    } else {
        0
    };
    let yp = if y < ComponentVideoOut::HEIGHT - 1 {
        y + 1
    } else {
        0
    };

    if grid[ym][xm] == Cell::Alive {
        neighbors += 1;
    }
    if grid[ym][x] == Cell::Alive {
        neighbors += 1;
    }
    if grid[ym][xp] == Cell::Alive {
        neighbors += 1;
    }

    if grid[y][xm] == Cell::Alive {
        neighbors += 1;
    }
    if grid[y][xp] == Cell::Alive {
        neighbors += 1;
    }

    if grid[yp][xm] == Cell::Alive {
        neighbors += 1;
    }
    if grid[yp][x] == Cell::Alive {
        neighbors += 1;
    }
    if grid[yp][xp] == Cell::Alive {
        neighbors += 1;
    }

    neighbors
}
